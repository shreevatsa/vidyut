import tempfile

import pytest


from vidyut.kosha import (
    Builder,
    Kosha,
    DhatuEntry,
    PratipadikaEntry,
    PadaEntry,
)

from vidyut.prakriya import (
    Gana,
    Purusha,
    Vacana,
    Krt,
    Prayoga,
    Pratipadika,
    Lakara,
    Linga,
    Vibhakti,
    Dhatu,
)


@pytest.fixture(scope="session")
def kosha():
    gam_entry = DhatuEntry(dhatu=Dhatu.mula("ga\\mx", Gana.Bhvadi), clean_text="gam")
    gacchati_tin = PadaEntry.Tinanta(
        dhatu_entry=gam_entry,
        prayoga=Prayoga.Kartari,
        lakara=Lakara.Lat,
        purusha=Purusha.Prathama,
        vacana=Vacana.Eka,
    )

    gacchati_sup = PadaEntry.Subanta(
        pratipadika_entry=PratipadikaEntry.Krdanta(dhatu_entry=gam_entry, krt=Krt.Satf),
        linga=Linga.Pum,
        vibhakti=Vibhakti.Saptami,
        vacana=Vacana.Eka,
    )

    with tempfile.TemporaryDirectory() as tempdir:
        b = Builder(tempdir)
        b.insert("gacCati", gacchati_tin)
        b.insert("gacCati", gacchati_sup)
        b.finish()

        return Kosha(tempdir)


def test_init(kosha):
    assert kosha


def test_init_fails_if_file_does_not_exist():
    with pytest.raises(OSError):
        with tempfile.TemporaryDirectory() as tempdir:
            _kosha = Kosha(tempdir)


@pytest.mark.skip("Not implemented yet")
def test_contains(kosha):
    assert "Bavati" not in kosha
    assert "gacCati" in kosha


@pytest.mark.skip("Not implemented yet")
def test_contains_prefix(kosha):
    for prefix in ["", "g", "ga", "gac", "gacC", "gacCa", "gacCat", "gacCati"]:
        assert kosha.contains_prefix(prefix)

    assert not kosha.contains_prefix("gacCati2")


@pytest.mark.skip("Not implemented yet")
def test_get_all(kosha):
    gam_entry = DhatuEntry(dhatu=Dhatu.mula("ga\\mx", Gana.Bhvadi), clean_text="gam")
    gacchati_tin = PadaEntry.Tinanta(
        dhatu_entry=gam_entry,
        prayoga=Prayoga.Kartari,
        lakara=Lakara.Lat,
        purusha=Purusha.Prathama,
        vacana=Vacana.Eka,
    )

    gacchati_sup = PadaEntry.Subanta(
        pratipadika_entry=PratipadikaEntry.Krdanta(dhatu_entry=gam_entry, krt=Krt.Satf),
        linga=Linga.Pum,
        vibhakti=Vibhakti.Saptami,
        vacana=Vacana.Eka,
    )

    assert len(kosha.get_all("Bavati")) == 0
    assert len(kosha.get_all("gacCati")) == 2

    [tin, sup] = kosha.get_all("gacCati")
    assert tin == gacchati_tin
    assert sup == gacchati_sup
