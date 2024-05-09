use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, help = "The HTML file to parse")]
    pub html: Option<String>,

    #[arg(long, help = "The CSS file to parse")]
    pub css: Option<String>,
}
