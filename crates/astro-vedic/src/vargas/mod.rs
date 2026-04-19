mod drekkana;
mod navamsa;

pub use drekkana::*;
pub use navamsa::*;

use crate::Rashi;

const RASHIS: [Rashi; 12] = [
    Rashi::Mesha,
    Rashi::Vrishabha,
    Rashi::Mithuna,
    Rashi::Karka,
    Rashi::Simha,
    Rashi::Kanya,
    Rashi::Tula,
    Rashi::Vrischika,
    Rashi::Dhanu,
    Rashi::Makara,
    Rashi::Kumbha,
    Rashi::Meena,
];

pub(crate) fn rashi_from_index(index: usize) -> Rashi {
    RASHIS[index % RASHIS.len()]
}

pub(crate) fn rashi_index(rashi: Rashi) -> usize {
    RASHIS
        .iter()
        .position(|candidate| *candidate == rashi)
        .expect("rashi table must contain every enum variant")
}
