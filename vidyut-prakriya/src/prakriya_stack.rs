use crate::prakriya::{Config, Prakriya, RuleChoice};
use std::error::Error;

/// Explores all optional derivations for some input.
///
/// Many of the rules in the Ashtadhyayi are optional, and by accepting or declining these optional
/// rules, we create different final results. `PrakriyaStack` manages the work required in finding
/// and exploring the various combinations of optional rules.
#[derive(Default)]
pub(crate) struct PrakriyaStack {
    /// Completed prakriyas.
    prakriyas: Vec<Prakriya>,
    /// Combinations of optional rules that we have yet to try.
    paths: Vec<Vec<RuleChoice>>,
}

impl PrakriyaStack {
    /// Creates an empty `PrakriyaStack`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new `Prakriya` according to upstream options.
    fn new_prakriya(rule_choices: Vec<RuleChoice>, log_steps: bool) -> Prakriya {
        Prakriya::with_config(Config {
            rule_choices,
            log_steps,
        })
    }

    /// Finds all variants of the given derivation function.
    ///
    /// `derive` should accept an empty `Prakriya` and mutate it in-place.
    pub fn find_all(
        &mut self,
        derive: impl Fn(&mut Prakriya) -> Result<(), Box<dyn Error>>,
        log_steps: bool,
    ) {
        let mut p_init = Self::new_prakriya(vec![], log_steps);
        // TODO: handle errors better.
        if derive(&mut p_init).is_ok() {
            self.add_prakriya(p_init, &[]);
        }

        while let Some(path) = self.pop_path() {
            let mut p = Self::new_prakriya(path.clone(), log_steps);
            if derive(&mut p).is_ok() {
                self.add_prakriya(p, &path);
            }
        }
    }

    /// Adds a prakriya to the result set and adds new paths to the stack.
    ///
    /// We find new paths as follows. Suppose our initial prakriya followed the following path:
    ///
    /// > Accept(A), Accept(B), Accept(C)
    ///
    /// We then add one candidate path for each alternate choice we could have made:
    ///
    /// > Decline(A)
    /// > Accept(A), Decline(B)
    /// > Accept(A), Accept(B), Decline(C)
    ///
    /// Suppose we then try `Decline(A)` and make the following choices:
    ///
    /// > Decline(A), Accept(B), Accept(D)
    ///
    /// After this, adding an `Accept(A) path to the stack would be a mistake, as it would cause an
    /// infinite loop. Instead, we freeze our initial decision to use `Decline(A)` and add only the
    /// following paths:
    ///
    /// > Decline(A), Decline(B)
    /// > Decline(A), Accept(B), Decline(D)
    fn add_prakriya(&mut self, p: Prakriya, initial_choices: &[RuleChoice]) {
        let choices = p.rule_choices();
        let offset = initial_choices.len();
        for i in offset..choices.len() {
            let mut path = choices[..=i].to_vec();

            // Swap the last choice.
            let i = path.len() - 1;
            path[i] = match path[i] {
                RuleChoice::Accept(code) => RuleChoice::Decline(code),
                RuleChoice::Decline(code) => RuleChoice::Accept(code),
            };

            self.paths.push(path);
        }
        self.prakriyas.push(p);
    }

    /// Pops an unexplored choice path from the stack.
    fn pop_path(&mut self) -> Option<Vec<RuleChoice>> {
        self.paths.pop()
    }

    /// Returns all of the prakriyas this stack has found. This consumes the stack.
    pub fn prakriyas(self) -> Vec<Prakriya> {
        self.prakriyas
    }
}
