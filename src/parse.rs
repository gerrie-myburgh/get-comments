use std::collections::HashSet;
use std::fs::{File, create_dir_all, remove_dir_all};
use std::io::{self, BufRead, BufWriter, Error, ErrorKind, Write};
use walkdir::WalkDir;

type Value = String;
type CommentStart = String;

#[derive(Default, PartialEq)]
enum State {
    #[default]
    CODE,
    COMMENT,
    ERROR,
}

#[derive(Default)]
pub struct Comments {
    current_state: State,
    comment: Vec<Value>,
    start_of_comment: CommentStart,
    log_file: Option<io::BufWriter<File>>,
    comment_block_names: HashSet<String>,
    current_comment_name: String,
    line_counter: u16,
    comment_line_start: u16,
}

impl Comments {
    ////EPIC comment.ITEM write to file
    ////# Write Comment Block To File
    ////Create the file path and write out the comment block to the file.
    fn write_out_to_file(
        &self,
        folder_prefixes: &Vec<&str>,
        file_path_and_name: &str,
        first_line: &str,
        lines: &Vec<String>,
    ) -> Result<(), std::io::Error> {
        // file_name is a '.' delimited slice. Each subslice is a folder starting
        // from the current `working folder
        let mut path: Vec<&str> = file_path_and_name.split(".").collect();
        if let Err(message) = self.is_valid_folder_path(folder_prefixes, file_path_and_name) {
            return Err(Error::new(ErrorKind::Other, message));
        }

        if let Some(file) = path.pop() {
            create_dir_all(path.join("/"))?;
            path.push(file);
            let the_path = path.join("/");
            let file = File::create(format!("{the_path}.md"))?;
            let mut writer = BufWriter::new(file);

            writeln!(writer, "{}", first_line)?;
            for line in lines {
                writeln!(writer, "{}", line)?;
            }
        }
        Ok(())
    }

    // check if the file-path and file name is valid wrt to the folder prefixes
    fn is_valid_folder_path(
        &self,
        folder_prefixes: &Vec<&str>,
        file_path_and_name: &str,
    ) -> Result<(), String> {
        let path: Vec<&str> = file_path_and_name.split(".").collect();
        if path.is_empty() {
            return Err(
                "There is no file path in the first line of the comment block.".to_string(),
            );
        }
        if path.len() > folder_prefixes.len() + 1 {
            return Err("Path is longer than what is allowed.".to_string());
        }

        let comment_name = file_path_and_name;
        if self.comment_block_names.contains(comment_name) {
            return Err("Comment block name must be unique in code base.".to_string());
        }

        let prefixes: Vec<_> = path[1..].iter().zip(folder_prefixes).collect();
        for item in prefixes {
            if !item.0.starts_with(item.1) {
                return Err(format!("Invalid folder prefix [{}] [{}].", item.0, item.1));
            }
        }
        Ok(())
    }

    ////EPIC comment.ITEM start
    ////# Parse Comment Start
    ////This is the first line of a comment start. Check that this line has a name
    ////and that this name is unique. Record the location in the source where the
    ////comment starts.
    fn parse_comment_start(&mut self, line: &str) -> Result<(), String> {
        let comment_name = line[self.start_of_comment.len()..].trim();
        self.comment_line_start = self.line_counter + 1;
        self.current_comment_name = comment_name.to_string();
        Ok(())
    }

    ////EPIC comment.ITEM line
    ////# Parse a Comment Line
    ////If the comment line is the first line in a comment then record as then check as first comment
    ////
    ////else record the line as part of the body of the comment.
    fn parse_comment(&mut self, line: &str) -> Result<(), String> {
        if self.current_state == State::CODE {
            self.current_state = State::COMMENT;
            self.parse_comment_start(line)?;
        } else {
            let comment_line = line[self.start_of_comment.len()..].to_string();
            self.comment.push(comment_line.to_string());
        }
        Ok(())
    }

    ////EPIC comment.ITEM file
    ////# Parse File For Comments
    ////Open the file iff it exist. Read the file line by line and check if the line
    ////is a comment. If the line is a comment then record it as a comment
    fn parse_file(
        &mut self,
        file_name: &str,
        doc_root: &str,
        folder_prefix: &str,
    ) -> Result<(), std::io::Error> {
        let file = File::open(file_name)?;
        let buf_reader = io::BufReader::new(file);
        let folder_prefixes: Vec<&str> = folder_prefix.split(".").collect();
        for line in buf_reader.lines() {
            let line = line?;
            let potential_comment_line = line.trim();
            if potential_comment_line.starts_with(self.start_of_comment.as_str()) {
                if let Err(message) = self.parse_comment(potential_comment_line) {
                    self.current_state = State::ERROR;
                    if self.log_file.is_some() {
                        let log = self.log_file.as_mut().unwrap();
                        log.write_all(message.as_bytes())?;
                    } else {
                        println!("{message}");
                    }
                }
            } else {
                if self.current_state == State::COMMENT {
                    self.current_state = State::CODE;
                    if self.comment.len() > 0 {
                        let first_line = format!(
                            "[SOURCE FILE:](file:///{file_name}) LINE: {}\n",
                            self.comment_line_start
                        );
                        //let comment = self.comment;
                        self.write_out_to_file(
                            &folder_prefixes,
                            format!("{doc_root}.{}", self.current_comment_name).as_str(),
                            first_line.as_str(),
                            &self.comment,
                        )?;
                        self.comment_block_names
                            .insert(self.current_comment_name.clone());
                        self.comment.clear();
                    }
                }
            }
            self.line_counter += 1u16;
        }
        Ok(())
    }

    pub fn comment_in_files(
        &mut self,
        folder_name: &str,
        doc_root: &str,
        start: &str,
        folder_prefixes: &str,
        file_extension: &str,
    ) {
        let _ = remove_dir_all(doc_root);
        self.start_of_comment = start.to_string();
        self.current_state = State::CODE;

        for entry in WalkDir::new(folder_name)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_name = entry.file_name().to_string_lossy();
            if entry.file_type().is_file() && file_name.ends_with(file_extension) {
                if let Some(name) = entry.path().to_str() {
                    self.line_counter = 1u16;
                    if let Err(_error) = self.parse_file(name, doc_root, folder_prefixes) {
                        println!("{_error:?}");
                    } else {
                        if self.current_state == State::ERROR {
                            println!("Error occurred while parsing file: {}", name);
                        }
                        // to do log None case as file dissapeared after getting name
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[test]
fn test_if_file_path_is_valid() {
    let mut comments = Comments::default();
    let path = &vec!["EPIC", "ITEM", "TEST"];
    if let Err(error) = comments.is_valid_folder_path(path, "EPIC epic.ITEM item.TEST test") {
        println!("{error}");
    }
    comments.current_comment_name = "EPIC epic.ITEM item.TEST test".to_string();
    comments
        .comment_block_names
        .insert(comments.current_comment_name.clone());
    if let Err(error) = comments.is_valid_folder_path(path, "EPIC epic.ITEM item.TEST test") {
        println!("{error}");
    }
}
