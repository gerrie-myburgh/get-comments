mod parse;
use cli_command::parse_command_line;

////EPIC comment
////# Get Comments In Rust Files
////Scan the source folder recursivly for rust source files and extract all the business rules
////comment blocks then put these in .md files.
fn main() {
    if let Ok(cli) = parse_command_line() {
        let some_dir = cli.get_argument("dir");
        let some_work = cli.get_argument("work");
        let some_start = cli.get_argument("start");
        let some_path = cli.get_argument("path");
        let some_extension = cli.get_argument("ext");

        if some_dir.is_some()
            && some_work.is_some()
            && some_start.is_some()
            && some_path.is_some()
            && some_extension.is_some()
        {
            let mut comment_parser = parse::Comments::default();
            comment_parser.comment_in_files(
                some_dir.unwrap(),
                some_work.unwrap(),
                some_start.unwrap(),
                some_path.unwrap(),
                some_extension.unwrap(),
            );
        } else {
            println!(
                "Missing command line -dir source_folder -work document_root -start comment_start -path legal_folder_prefix -ext file_extension"
            )
        }
    }
}
