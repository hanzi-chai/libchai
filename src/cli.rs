use clap::Parser;

#[derive(Parser)]
#[command(name = "汉字自动拆分系统")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(default_value_t = String::from("config.yaml"))]
    pub config: String,

    #[arg(short, long, value_name = "FILE", default_value_t = String::from("elements.txt"))]
    pub elements: String,
}
