//! Segments Sanskrit phrases into separate words with their morphological analysis.
use log::{debug, log_enabled, Level};
use priority_queue::PriorityQueue;
use std::collections::HashMap;
use std::error::Error;

use crate::config::Config;
use crate::normalize_text::normalize;
use crate::sandhi;
use crate::sandhi::Sandhi;
use crate::scoring::Model;
use crate::sounds;
use crate::strict_mode;
use vidyut_kosha::semantics::Pada;
use vidyut_kosha::Kosha;

/// Represnts a Sanskrit word and its semantics.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Word {
    pub text: String,
    pub semantics: Pada,
}

impl Word {
    /// Get the word's root/stem.
    pub fn lemma(&self) -> String {
        self.semantics.lemma()
    }
}

/// Represents an in-progress segment of a phrase.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Phrase {
    /// The words that we've recognized so far.
    pub words: Vec<Word>,
    /// The text we still need to process.
    pub remaining: String,
    /// The score associated with this in-progress solution.
    pub score: i32,
}

impl Phrase {
    /// Create a new state.
    pub const fn new(text: String) -> Self {
        Self {
            words: Vec::new(),
            remaining: text,
            // log_10(1) = 0
            score: 0,
        }
    }
}

/// A Sanskrit segmenter.
pub struct Segmenter {
    /// Sandhi rules. The segmenter uses these rules to exhaustively split a Sanskrit expression
    /// and find candidate words.
    sandhi: Sandhi,
    /// A lexicon of Sanskrit words. The segmenter uses this lexicon to examine a Sanskrit
    /// substring and test whether or not it is a valid Sanskrit word.
    lexicon: Kosha,
    /// A scoring model. The segmenter uses this model to score candidate solutions and prioritize
    /// solutions that are the most promising.
    model: Model,
}

impl Segmenter {
    /// Creates a segmenter from the given input data.
    pub fn new(config: Config) -> Result<Self, Box<dyn Error>> {
        Ok(Segmenter {
            sandhi: Sandhi::from_csv(config.sandhi()).expect("Could not read sandhi rules."),
            lexicon: Kosha::new(config.lexicon()).expect("Could not read lexicon."),
            model: Model::new(&config.model_lemma_counts(), &config.model_transitions())?,
        })
    }

    pub fn lexicon(&self) -> &Kosha {
        &self.lexicon
    }

    /// Segments the given text.
    ///
    /// `raw_text` should be an SLP1 string.
    pub fn segment(&self, raw_text: &str) -> Vec<Word> {
        segment(raw_text, self).expect("Is OK")
    }
}

// FIXME: better as an iterator, but hard to implement. For now, update statefully then iterate in
// caller.
fn analyze_pada(
    text: &str,
    split: &sandhi::Split,
    segmenter: &Segmenter,
    cache: &mut HashMap<String, Vec<Pada>>,
) -> Result<(), Box<dyn Error>> {
    if !cache.contains_key(text) {
        let res: Result<Vec<Pada>, _> = segmenter
            .lexicon
            .get_all(text)
            .iter()
            .map(|p| segmenter.lexicon.unpack(p))
            .collect();
        let mut res = res?;

        // Add the option to skip an entire chunk. (For typos, junk, etc.)
        if split.is_end_of_chunk || text.starts_with(|c| !sounds::is_sanskrit(c)) {
            res.push(Pada::None);
        }

        cache.insert(text.to_string(), res);
    };
    Ok(())
}

#[allow(dead_code)]
fn debug_print_phrase(p: &Phrase) {
    if log_enabled!(Level::Debug) {
        for word in &p.words {
            debug!("- {} {:?}", word.text, word.semantics);
        }
        debug!("score={}", p.score);
    }
}

#[allow(dead_code)]
fn debug_print_stack(pq: &PriorityQueue<Phrase, i32>) {
    if log_enabled!(Level::Debug) {
        debug!("Stack:");

        // The queue isn't sorted by default. So, sort from highest to lowest priotity.
        let mut words: Vec<(&Phrase, &i32)> = pq.iter().collect();
        words.sort_by(|x, y| y.1.cmp(x.1));

        for (i, (s, score)) in words.iter().enumerate() {
            let words: Vec<String> = s.words.iter().map(|x| x.text.clone()).collect();
            debug!("{}: \"{:?}\" + \"{}\" ({})", i, words, s.remaining, score);
        }
        debug!("-------------------");
    }
}

#[allow(dead_code)]
fn debug_print_viterbi(v: &HashMap<String, HashMap<String, Phrase>>) {
    if log_enabled!(Level::Debug) {
        debug!("Viterbi:");
        for (key1, entries) in v.iter() {
            for (key2, state) in entries.iter() {
                let words: Vec<String> = state.words.iter().map(|x| x.text.clone()).collect();
                debug!("(`{}`, {}) -> {:?} : {}", key1, key2, words, state.score);
            }
        }
        debug!("-------------------");
    }
}

/// Segments the given text.
///
/// # Arguments:
/// - `raw_text` - a text string in SLP1.
///
/// The segmenter makes a best effort to understand the input as valid Sanskrit text, even if it
/// contains typos or other content that is not valid Sanskrit.
fn segment(raw_text: &str, ctx: &Segmenter) -> Result<Vec<Word>, Box<dyn Error>> {
    let text = normalize(raw_text);
    let mut pq = PriorityQueue::new();
    let mut word_cache: HashMap<String, Vec<Pada>> = HashMap::new();

    // viterbi_cache[remainder][state] = the best result that ends with $state and has $remainder
    // text remaining in the input.
    let mut viterbi_cache: HashMap<String, HashMap<String, Phrase>> = HashMap::new();

    let initial_state = Phrase::new(text);
    let score = initial_state.score;
    pq.push(initial_state, score);

    while !pq.is_empty() {
        debug_print_stack(&pq);
        // debug_print_viterbi(&viterbi_cache);

        // Pop the best solution remaining.
        let (cur, cur_score) = pq.pop().unwrap();

        // The best solution remaining is complete, so we can stop here.
        //
        // Our current scoring model is a probabilistic model that adjusts the probability of a
        // solution by multiplying it by other probabilities. Since a probability is at most 1, a
        // partial score in a probabilistic model can never increase; in practice, it will only
        // decrease as more and more terms are added.
        //
        // In other words, a solution's score can only decrease as we add more words to it.
        //
        // If we see a complete solution in our priority queue with score C, we thus know that all
        // solutions following it both (a) have a score equal or lower to C due to the nature of
        // priority queues and (b) cannot possibly produce a result better than C per our result
        // above.
        //
        // So once we find a finished solution in our priority queue, we can suspend execution.
        //
        // NOTE: this doesn't hold if using an actual Viterbi algorithm as we can suspend only once
        // we've seen each of our N possible states.
        if cur.remaining.is_empty() {
            break;
        }

        // Non-Sanskrit token: emit and continue.
        if cur.remaining.starts_with(|c| !sounds::is_sanskrit(c)) {
            let mut new = match cur.remaining.split_once(' ') {
                Some((first, second)) => {
                    let mut new = Phrase {
                        words: cur.words.clone(),
                        remaining: second.to_string(),
                        // HACK: this is buggy -- scoring based on cur score set here?
                        score: cur_score,
                    };
                    new.words.push(Word {
                        text: first.to_string(),
                        semantics: Pada::None,
                    });
                    new
                }
                None => {
                    let mut new = Phrase {
                        words: cur.words.clone(),
                        remaining: "".to_string(),
                        // HACK: this is buggy -- scoring based on cur score set here?
                        score: cur_score,
                    };
                    new.words.push(Word {
                        text: cur.remaining.to_string(),
                        semantics: Pada::None,
                    });
                    new
                }
            };

            new.score = ctx.model.score(&new);
            viterbi_cache
                .entry(new.remaining.clone())
                .or_insert_with(HashMap::new)
                .insert("STATE".to_string(), new.clone());

            let new_score = new.score;
            pq.push(new, new_score);
            continue;
        }

        // A clumsy workaround because I'm not sure how to set up the iterator types here.
        let no_results = Vec::new();

        for split in ctx.sandhi.split_all(&cur.remaining) {
            if !split.is_valid() || split.is_recursive(&cur.remaining) {
                continue;
            }

            let first = &split.first;
            let second = &split.second;
            analyze_pada(first, &split, ctx, &mut word_cache)?;

            for semantics in word_cache.get(first).unwrap_or(&no_results) {
                if !strict_mode::is_valid_word(&cur, &split, semantics) {
                    continue;
                }

                let mut new = Phrase {
                    words: cur.words.clone(),
                    remaining: second.to_string(),
                    // HACK: this is buggy -- scoring based on cur score set here?
                    score: cur_score,
                };
                new.words.push(Word {
                    text: first.clone(),
                    semantics: semantics.clone(),
                });
                new.score = ctx.model.score(&new);

                // Use state "STATE" for now since we don't have any states implemented.
                let maybe_rival = viterbi_cache
                    .entry(new.remaining.clone())
                    .or_insert_with(HashMap::new)
                    .get("STATE");
                let new_score = new.score;
                if let Some(rival) = maybe_rival {
                    if rival.score >= new.score {
                        continue;
                    }
                };
                viterbi_cache
                    .entry(new.remaining.clone())
                    .or_insert_with(HashMap::new)
                    .insert("STATE".to_string(), new.clone());
                pq.push(new, new_score);
            }
        }
    }

    // Return the best result we could find above.
    if let Some(solutions) = viterbi_cache.get("") {
        if let Some(best) = solutions.values().max_by_key(|s| s.score) {
            return Ok(best.words.clone());
        }
    }
    Ok(Vec::new())
}
