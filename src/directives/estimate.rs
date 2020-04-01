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
