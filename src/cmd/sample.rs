use std::io;

use byteorder::{ByteOrder, LittleEndian};
use csv;
use rand::rngs::StdRng;
use rand::{self, Rng, SeedableRng};

use crate::config::{Config, Delimiter};
use crate::index::Indexed;
use crate::CliResult;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_sample_size: u64,
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
    #[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
    #[arg(long = "seed", value_name = "arg")]
    pub flag_seed: Option<usize>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers);
    let sample_size = args.arg_sample_size;

    let mut wtr = Config::new(&args.flag_output.clone()).writer()?;
    let sampled = match rconfig.indexed()? {
        Some(mut idx) if do_random_access(sample_size, idx.count()) => {
            rconfig.write_headers(&mut *idx, &mut wtr)?;
            sample_random_access(&mut idx, sample_size)?
        }
        _ => {
            let mut rdr = rconfig.reader()?;
            rconfig.write_headers(&mut rdr, &mut wtr)?;
            sample_reservoir(&mut rdr, sample_size, args.flag_seed)?
        }
    };
    for row in sampled.into_iter() {
        wtr.write_byte_record(&row)?;
    }
    Ok(wtr.flush()?)
}

fn sample_random_access<R, I>(
    idx: &mut Indexed<R, I>,
    sample_size: u64,
) -> CliResult<Vec<csv::ByteRecord>>
where
    R: io::Read + io::Seek,
    I: io::Read + io::Seek,
{
    let mut all_indices = (0..idx.count()).collect::<Vec<_>>();
    use rand::seq::SliceRandom;
    let mut rng = ::rand::thread_rng();
    all_indices.shuffle(&mut rng);

    let mut sampled = Vec::with_capacity(sample_size as usize);
    for i in all_indices.into_iter().take(sample_size as usize) {
        idx.seek(i)?;
        sampled.push(idx.byte_records().next().unwrap()?);
    }
    Ok(sampled)
}

fn sample_reservoir<R: io::Read>(
    rdr: &mut csv::Reader<R>,
    sample_size: u64,
    seed: Option<usize>,
) -> CliResult<Vec<csv::ByteRecord>> {
    // The following algorithm has been adapted from:
    // https://en.wikipedia.org/wiki/Reservoir_sampling
    let mut reservoir = Vec::with_capacity(sample_size as usize);
    let mut records = rdr.byte_records().enumerate();
    for (_, row) in records.by_ref().take(reservoir.capacity()) {
        reservoir.push(row?);
    }

    // Seeding rng
    let mut rng: StdRng = match seed {
        None => StdRng::from_rng(rand::thread_rng()).unwrap(),
        Some(seed) => {
            let mut buf = [0u8; 32];
            LittleEndian::write_u64(&mut buf, seed as u64);
            SeedableRng::from_seed(buf)
        }
    };

    // Now do the sampling.
    for (i, row) in records {
        let random = rng.gen_range(0..i + 1);
        if random < sample_size as usize {
            reservoir[random] = row?;
        }
    }
    Ok(reservoir)
}

fn do_random_access(sample_size: u64, total: u64) -> bool {
    sample_size <= (total / 10)
}
