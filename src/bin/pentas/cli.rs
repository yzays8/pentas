use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, help = "The HTML file to parse")]
    pub html: Option<String>,

    #[arg(long, conflicts_with = "html", help = "The CSS file to parse")]
    pub css: Option<String>,

    #[arg(long, help = "Display all intermediate steps")]
    pub trace: bool,

    #[arg(long, help = "Do not render the output")]
    pub no_rendering: bool,
}
