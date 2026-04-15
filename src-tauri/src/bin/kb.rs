fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match meme_retriever_lib::kb::maintenance::execute_cli(&args) {
        Ok(output) => {
            println!("{output}");
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
