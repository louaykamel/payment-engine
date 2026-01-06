pub(crate) use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "payment-engine",
    author,
    version,
    about = "A simple toy payments engine",
    long_about = None,
    after_help = "OUTPUT:\n    Results are printed to stdout in CSV format.\n    Use shell redirection to save to a file:\n\n    payment-engine transactions.csv > accounts.csv"
)]
pub struct Args {
    /// Path to the input transactions CSV file
    #[arg(
        index = 1,
        value_name = "FILE",
        help = "Input CSV file with columns: type, client, tx, amount"
    )]
    pub input_file: PathBuf,
}
