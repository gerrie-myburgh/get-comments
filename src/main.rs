mod parse;
use cli_command::parse_command_line;

const VERSION: &str = "1.0.1";
//#EPIC Get Lines
//#[$FILE$](file://$FILE$) LINE: $LINE$
//#
//## Get lines from text files and put the line blocks into Markdown files.
//#A _line block_ is any number number of consecutive lines that starts with the `start` string,
//#an example is a start string of `//#` and an example line ` //# some text...`.
//#
//#The start line of a block must contain the name of the path and file name along with the block number.
//#The block number will determine the location in the md file where line blocks with the same path and file
//#name. This block number will be an unsigned number.
//#
//### The process flow is as follow:
//#
//# 1. [[docs/EPIC Get Lines/ITEM Get Line Blocks in all files.md]]
//#    Traverse the folder structure and get all the files with the given extension. Then process files one by one.
//# 2. [[docs/EPIC Get Lines/ITEM Parse file for line blocks.md]]
//#    Take the current file file and get all the comment lines from the file ad place it in history.
//# 3. [[docs/EPIC Get Lines/ITEM Write out all of the history.md]]
//#    Once all of the files is processed then write out the comment one by one to the Markdown files.
//# 4. [[docs/EPIC Get Lines/ITEM Write the comment lines to the file path and name.md]]
//#    Take the current comment block and write it out to the Markdown file.
//
//#EPIC Get Lines.ITEM Improve error messaging
//##Improve error messages by making them descriptive from user point of view.
//#
//# 1. Change error messages strings
//# 2. Prepend error code to error messages eg E00: or E01: ...
//# 3. Add command line parameter to define output files extension and use it
fn main() {
    if let Ok(cli) = parse_command_line() {
        let get_version = cli.contains_argument("ver");
        let some_dir = cli.get_argument("dir");
        let some_work = cli.get_argument("work");
        let some_start = cli.get_argument("start");
        let some_path = cli.get_argument("path");
        let some_extension = cli.get_argument("ext");
        let some_destination_extension = cli.get_argument("dest");

        if get_version {
            println!("{}", VERSION);
            return;
        }
        if some_dir.is_some()
            && some_work.is_some()
            && some_start.is_some()
            && some_path.is_some()
            && some_extension.is_some()
            && some_destination_extension.is_some()
        {
            let mut comment_parser = parse::Comments::default();
            comment_parser.comment_in_files(
                some_dir.unwrap(),
                some_work.unwrap(),
                some_start.unwrap(),
                some_path.unwrap(),
                some_extension.unwrap(),
                some_destination_extension.unwrap(),
            );
        } else {
            println!(
                "command line -dir source_folder -work document_root -start comment_start -path legal_folder_prefix -ext file_extension -dest destination_files_extension"
            )
        }
    }
}
