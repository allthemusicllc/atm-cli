// estimate.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use crate::{
    cli::CliDirective,
    directives::{
        EstimateTarDirective,
        EstimateTarGzDirective,
    },
};

/// Maximum number of melodies to use for size estimatation.
/// Only used for tar_gz and batch backends.
pub(crate) const MAX_SIM_NUM_MELODIES: u64 = 200000;

/// Given total number of melodies, the simulated number of melodies,
/// and resulting size estimate, generate the total estimated file size.
pub(crate) fn gen_sim_file_size(sim_num_melodies: u64, num_melodies: u64, size_estimate: u64) -> u64 {
    if sim_num_melodies == num_melodies {
        size_estimate
    } else if sim_num_melodies == MAX_SIM_NUM_MELODIES {
        ((num_melodies as f64 / MAX_SIM_NUM_MELODIES as f64).ceil() as u64) * size_estimate
    } else {
        size_estimate * 5
    }
}

/// Generate number of melodies to use for estimation.
pub(crate) fn gen_sim_num_melodies(num_melodies: u64) -> u64 {
    if num_melodies <= MAX_SIM_NUM_MELODIES {
        num_melodies
    } else {
        std::cmp::min((num_melodies as f64 * 0.20).floor() as u64, MAX_SIM_NUM_MELODIES)
    }
}

/// Pad value to align to block size. Default block size is `512` (bytes).
pub(crate) fn pad_value_to_block(value: u64, block_size: Option<u64>) -> u64 {
    let block_size = block_size.unwrap_or(512);
    if let Some(padding) = value.checked_rem_euclid(block_size) {
        value + padding
    } else {
        value
    }
}

/****************************
***** EstimateDirective *****
****************************/

#[derive(structopt::StructOpt)]
#[structopt(about=concat!("Estimate output size of storage backends ",
                          "to help make informed decisions about which to use."))]
pub enum EstimateDirective {
    #[structopt(name="tar", about="Estimate output size of Tar file storage backend")]
    EstimateTar(EstimateTarDirective),
    #[structopt(
        name="tar_gz",
        about="Estimate output size of Gzip-compressed Tar file storage backend")]
    EstimateTarGz(EstimateTarGzDirective),
}

impl CliDirective for EstimateDirective {
    fn run(self) {
        match self {
            Self::EstimateTar(d) => d.run(),
            Self::EstimateTarGz(d) => d.run(),
        }
    }
}
