use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, value_name = "HTML", help = "The HTML file to parse in CLI mode")]
    pub no_window_html: Option<String>,

    #[arg(
        long,
        value_name = "CSS",
        conflicts_with = "no_window_html",
        help = "The CSS file to parse in CLI mode"
    )]
    pub no_window_css: Option<String>,

    #[arg(long, help = "Display all intermediate steps")]
    pub trace: bool,
}
