use clap::{Parser, Subcommand};
use clipocr_rs::baidu_ocr_api::{self as ocr, OcrApi};
use clipocr_rs::clipboard::{get_img_base64_from_clipboard, set_clipboard};
use rustyline::Editor;


// get platform specifc newline symbol
// windows, macos and linux
// if non matched, use \n
fn get_newline() -> &'static str {
    let os = std::env::consts::OS;
    match os {
        "windows" => "\r\n",
        "macos" => "\r",
        "linux" => "\n",
        _ => "\n",
    }
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    General,
    Accurate,
    GenerateConfig,
}

fn read_content(e: &mut Editor<()>,prompt: &str) -> String {
    let line = e.readline(prompt).unwrap();
    match line.trim() {
        "" => read_content(e, prompt),
        _ => line,
    }
}
fn get_conf_and_state_file() -> (std::path::PathBuf, std::path::PathBuf) {
    let mut conf_dir = dirs::config_dir().unwrap();
    conf_dir.push("clipocr-rs");
    // create if not exist
    std::fs::create_dir_all(&conf_dir).unwrap();
    let conf_file = conf_dir.join("config.yaml");
    let state_file = conf_dir.join("state.yaml");
    (conf_file, state_file)
}
fn gen_config() {
    let (conf_file, _) = get_conf_and_state_file();

    // prompt to input appid, apikey and secretkey
    let mut ed = Editor::<()>::new().expect("failed to create rustyline editor");
    let appid = read_content(&mut ed, "appid: ");
    let apikey = read_content(&mut ed, "apikey: ");
    let secret_key = read_content(&mut ed, "secret key: ");
    let cfg = ocr::OcrConfig::new(appid, apikey, secret_key);
    cfg.to_yaml(&conf_file.to_str().unwrap());
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match &cli.subcmd {
        Commands::GenerateConfig => {
            gen_config();
            // exit
            std::process::exit(0);
        }
        _ => {}
    };

    let img = get_img_base64_from_clipboard();
    let (conf_file, state_file) = get_conf_and_state_file();
    let config = ocr::OcrConfig::from_yaml(&conf_file.to_str().unwrap());
    let state = config.get_valid_state(&state_file.to_str().unwrap()).await;
    let ocr_client = match &cli.subcmd {
        Commands::Accurate => {
            let client = ocr::BaiduOcrApis::AccurateBasic(ocr::BaiduAccurateBasic::from_state(&state));
            client
        }
        Commands::General => {
            let client = ocr::BaiduOcrApis::GeneralBasic(ocr::BaiduGeneralBasic::from_state(&state));
            client
        }
        _ => {unreachable!()}
    };
    let result = ocr_client.get_text_result(&img).await;
    let cliptext = result.join(get_newline());
    set_clipboard(&cliptext);
}
