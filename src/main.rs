use std::ops::RangeInclusive;
use std::path::PathBuf;

use clap::Parser as CliParser;
use winnow::Parser;

#[derive(CliParser, Debug)]
#[command(name = "PDF Page Extractor")]
#[command(author, version)]
#[command(
about = "Extracts given pages from an input PDF file and stores them in a new output PDF file.",
long_about = None,
)]
struct Cli {
    #[arg(value_hint = clap::ValueHint::FilePath)]
    input_file: PathBuf,

    #[arg(value_hint = clap::ValueHint::FilePath)]
    output_file: PathBuf,

    #[arg(value_parser = cli_pages_value_parser)]
    pages: Pages,
}

#[derive(Clone, Debug)]
struct Pages(Vec<usize>);

fn cli_pages_value_parser(input: &str) -> Result<Pages, anyhow::Error> {
    let result = parse_page_list.parse(input);

    match result {
        Ok(ranges) => Ok(Pages(
            ranges
                .into_iter()
                .flat_map(RangeInclusive::into_iter)
                .collect(),
        )),
        Err(err) => anyhow::bail!(err.to_string()),
    }
}

fn parse_page_list(input: &str) -> winnow::IResult<&str, Vec<RangeInclusive<usize>>> {
    winnow::combinator::repeat(
        0..,
        winnow::combinator::terminated(parse_page_range, winnow::combinator::opt(',')),
    )
    .parse_next(input)
}

fn parse_page_range(input: &str) -> winnow::IResult<&str, RangeInclusive<usize>> {
    let (input, range_start) =
        winnow::combinator::opt(winnow::ascii::digit1.parse_to()).parse_next(input)?;
    let (input, separator) = winnow::combinator::opt('-').parse_next(input)?;
    let (remainder, range_end) =
        winnow::combinator::opt(winnow::ascii::digit1.parse_to()).parse_next(input)?;

    let range = match (range_start, separator, range_end) {
        (Some(range_start), None, _) => RangeInclusive::new(range_start, range_start),
        (Some(range_start), Some(_), None) => RangeInclusive::new(range_start, usize::MAX),
        (None, Some(_), Some(range_end)) => RangeInclusive::new(usize::MIN, range_end),
        (Some(range_start), Some(_), Some(range_end)) => {
            RangeInclusive::new(range_start, range_end)
        }
        _ => {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::Error::new(remainder, winnow::error::ErrorKind::Fail),
            ));
        }
    };

    Ok((remainder, range))
}

// TODO: tests for the parsing

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    run_extract(&cli)
}

// TODO: remove references to the pages as well
fn run_extract(cli: &Cli) -> anyhow::Result<()> {
    if !cli.input_file.as_path().try_exists()? {
        anyhow::bail!("the given INPUT_FILE does not exist");
    }
    if cli.output_file.as_path().try_exists()? {
        anyhow::bail!("the given OUTPUT_FILE already exists, please remove it first")
    }

    let mut pdf = lopdf::Document::load(&cli.input_file)?;

    let pdf_page_count = pdf.page_iter().count();

    let range = 0..pdf_page_count;
    let pages_to_remove = range
        .into_iter()
        .filter(|page_number| !cli.pages.0.contains(page_number))
        .map(|page_number| page_number as u32)
        .collect::<Vec<u32>>();

    pdf.delete_pages(&pages_to_remove[..]);

    pdf.save(&cli.output_file)?;

    Ok(())
}
