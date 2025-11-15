use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{File, OpenOptions, create_dir_all, remove_dir_all};
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
pub struct Comments<'a> {
    folder_prefixes: Vec<&'a str>,
    current_state: State,
    comment_history: HashMap<String, BTreeMap<u16, Vec<String>>>,
    comment: Vec<Value>,
    start_of_comment: CommentStart,
    log_file: Option<io::BufWriter<File>>,
    comment_block_names: HashSet<String>,
    current_comment_name: String,
    line_counter: u16,
    comment_line_start: u16,
}

impl<'a> Comments<'a> {
    //#EPIC Get Lines.ITEM Write the comment lines to the file path and name [0]
    //#
    //## Write Comment Block To File
    //#Create the file path and write out the comment block to the file having file name.
    /// Writes comment lines to a markdown file in the specified hierarchical directory structure.
    ///
    /// This function takes a dot-delimited file path, creates the necessary directory structure,
    /// and appends comment lines to a markdown file. It's the core file output operation for
    /// the documentation generation system.
    ///
    /// # Process Flow:
    /// 1. **Path Validation**: Calls `is_valid_folder_path` to validate the hierarchical structure
    /// 2. **Directory Creation**: Creates all necessary directories in the path hierarchy
    /// 3. **File Preparation**: Opens the markdown file in append mode (creates if doesn't exist)
    /// 4. **Content Writing**: Writes all comment lines followed by a blank line
    /// 5. **Buffered Output**: Uses BufWriter for efficient file I/O operations
    ///
    /// # Path Processing:
    /// - **Input Format**: Dot-delimited path (e.g., "doc_root.EPIC.ITEM.TASK.Description")
    /// - **Directory Creation**: Converts dots to directory separators and creates folders
    /// - **File Naming**: The last component becomes the markdown filename
    /// - **Output Example**: "doc_root/EPIC/ITEM/TASK/Description.md"
    ///
    /// # Parameters:
    /// - `folder_prefixes`: Expected folder hierarchy for validation
    /// - `file_path_and_name`: Dot-delimited path where file should be created
    /// - `lines`: Vector of comment lines to write to the file
    ///
    /// # Returns:
    /// - `Ok(())` on successful file creation and writing
    /// - `Err(std::io::Error)` if directory creation, file opening, or writing fails
    ///
    /// # File Operations:
    /// - **Append Mode**: Files are opened in append mode to support multiple comment blocks
    /// - **Create Flag**: Files are created if they don't exist
    /// - **Buffered Writing**: Uses BufWriter for performance with multiple write operations
    /// - **Blank Line**: Adds a trailing blank line to separate comment blocks
    ///
    /// # Note:
    /// - This function is called by `write_history` for each comment block Sequence
    /// - Multiple Sequences of the same comment block are appended to the same file
    /// - The directory structure mirrors the hierarchical organization of comment blocks
    /// - File operations are atomic within this function call
    fn write_out_to_file(
        &self,
        folder_prefixes: &Vec<&str>,
        file_path_and_name: &str,
        lines: &Vec<String>,
    ) -> Result<(), std::io::Error> {
        // file_name is a '.' delimited slice. Each slice is a folder starting
        // from the current `working folder
        let mut path: Vec<&str> = file_path_and_name.split(".").collect();
        if let Err(message) = self.is_valid_folder_path(folder_prefixes, file_path_and_name) {
            return Err(Error::new(ErrorKind::Other, message));
        }

        if let Some(file) = path.pop() {
            create_dir_all(path.join("/"))?;
            path.push(file);
            let path_and_file_name = format!("{}.md", path.join("/"));
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(path_and_file_name)?;
            let mut writer = BufWriter::new(file);
            for line in lines {
                writeln!(writer, "{}", line)?;
            }
            writeln!(writer, "")?;
        }
        Ok(())
    }
    /// Extracts Sequence number from comment block names and returns the sanitized name.
    ///
    /// This function parses comment block names that follow the pattern "BlockName [N]"
    /// where N is a Sequence number in brackets at the end of the string. It extracts
    /// both the Sequence number and the base block name for separate handling.
    ///
    /// # Pattern Matching:
    /// - **Regex Pattern**: `r"\[\d+\]$"` - matches numbers in brackets at string end
    /// - **Examples**:
    ///   - "EPIC.Get Lines.ITEM Test Block [1]" → Sequence=1, name="EPIC.Get Lines.ITEM Test Block"
    ///   - "Simple Comment [42]" → Sequence=42, name="Simple Comment"
    ///   - "No Sequence" → Error: "No Sequence number exist in name of block"
    ///
    /// # Extraction Process:
    /// 1. **Regex Matching**: Finds Sequence number pattern at end of string
    /// 2. **Sequence Parsing**: Extracts number from brackets and converts to u16
    /// 3. **Validation**: Ensures Sequence number exists and is valid
    /// 4. **Name Sanitization**: Removes Sequence suffix to get clean block name
    ///
    /// # Parameters:
    /// - `a_string`: Comment block name string that may contain Sequence suffix
    ///
    /// # Returns:
    /// - `Ok((u16, String))` - Tuple containing (version_number, sanitized_block_name)
    /// - `Err(Error)` - If no Sequence number is found in the string
    ///
    /// # Error Conditions:
    /// - No Sequence number pattern found at the end of the string
    /// - Sequence number cannot be parsed as u16 (though regex ensures it's numeric)
    ///
    /// # Use Cases:
    /// - Used by `write_out_all_history` to separate Sequence from block name for storage
    /// - Enables multiple Sequences of the same comment block to be tracked and organized
    /// - Supports versioned documentation where blocks can be updated over time
    ///
    /// # Note:
    /// - The Sequence number must be at the very end of the string in brackets
    /// - The regex ensures only numeric values are accepted as Sequence numbers
    /// - This enables the system to maintain Sequence history for comment blocks
    /// - Sequence numbers are used to order comment blocks chronologically in output
    fn strip_number_in_str(&self, a_string: &String) -> Result<(u16, String), Error> {
        let version_of_block = Regex::new(r"\[\d+\]$").unwrap();
        let mut version_number: Option<u16> = None;
        if let Some(capture) = version_of_block.captures(a_string) {
            if let Some(matched) = capture.get(0) {
                if let Ok(version_num) = matched
                    .as_str()
                    .replace("[", "")
                    .replace("]", "")
                    .parse::<u16>()
                {
                    version_number = Some(version_num);
                }
            }
        }

        if version_number.is_none() {
            return Err(Error::new(
                ErrorKind::Other,
                "No Sequence number exist in name of block",
            ));
        }
        let block = version_of_block.replace_all(a_string, "");
        Ok((version_number.unwrap(), block.as_ref().to_string()))
    }
    /// Writes all accumulated comment blocks from history to their respective documentation files.
    ///
    /// This function serves as the final output phase of the documentation generation process,
    /// iterating through all comment blocks stored in `comment_history` and writing them to
    /// their designated markdown files in the documentation hierarchy.
    ///
    /// # Process Flow:
    /// 1. **Iteration**: Loops through all comment blocks organized by file path and Sequence
    /// 2. **File Writing**: For each comment block, calls `write_out_to_file` to create/append
    ///    to the corresponding markdown file
    /// 3. **Error Collection**: Accumulates any file writing errors without stopping the process
    /// 4. **Final Error Check**: Returns a single error if any file operations failed
    ///
    /// # Data Structure Navigation:
    /// - **Outer HashMap**: Keyed by file path (e.g., "doc_root.EPIC.ITEM")
    /// - **Inner BTreeMap**: Keyed by Sequence number, maintains comment blocks in Sequence order
    /// - **Value**: Vector of comment lines for each Sequence of a comment block
    ///
    /// # Error Handling Strategy:
    /// - **Non-blocking**: Continues processing all files even if some fail
    /// - **Aggregated Errors**: Collects all error messages into a single string
    /// - **Single Return**: Returns one comprehensive error if any failures occurred
    ///
    /// # Returns:
    /// - `Ok(())` if all comment blocks were successfully written to files
    /// - `Err(Error)` containing concatenated error messages if any file operations failed
    ///
    /// # Note:
    /// - This function is typically called at the end of `comment_in_files` after all
    ///   source files have been processed
    /// - The use of BTreeMap ensures comment blocks are written in Sequence order
    /// - File paths are constructed from the hierarchical comment block names
    /// - Multiple Sequences of the same comment block are written to the same file
    ///   in Sequence order
    fn write_history(&self) -> Result<(), Error> {
        let mut error_string = String::new();
        self.comment_history.iter().for_each(
            |blocks_to_write: (&String, &BTreeMap<u16, Vec<String>>)| {
                let file_name = blocks_to_write.0.as_str().trim();

                for (_, value) in blocks_to_write.1 {
                    if let Err(error) =
                        self.write_out_to_file(&self.folder_prefixes, file_name, value)
                    {
                        error_string = error.to_string()
                    }
                }
            },
        );
        if !error_string.is_empty() {
            Err(Error::new(ErrorKind::Other, error_string))
        } else {
            Ok(())
        }
    }
    /// Validates the folder path structure and naming conventions for comment blocks.
    ///
    /// This function performs comprehensive validation on comment block names to ensure
    /// they follow the required hierarchical structure and naming conventions before
    /// being processed for documentation generation.
    ///
    /// # Validation Rules:
    /// 1. **Path Structure**: The comment block name must be a dot-separated path
    ///    (e.g., "EPIC.ITEM.TASK.Description")
    /// 2. **Path Length**: The path cannot exceed the folder prefix hierarchy length + 1
    ///    (e.g., if folder_prefixes has 3 elements, path can have up to 4 elements)
    /// 3. **Uniqueness**: Comment block names must be unique across the entire codebase
    /// 4. **Prefix Matching**: Each path component (except the first) must start with
    ///    the corresponding folder prefix
    ///
    /// # Validation Process:
    /// 1. **Path Parsing**: Splits the dot-separated path into components
    /// 2. **Empty Check**: Ensures at least one path component exists
    /// 3. **Length Check**: Validates path doesn't exceed maximum allowed depth
    /// 4. **Uniqueness Check**: Verifies comment block name hasn't been used before
    /// 5. **Prefix Validation**: Ensures each path component matches folder prefix hierarchy
    ///
    /// # Parameters:
    /// - `folder_prefixes`: Expected folder hierarchy prefixes (e.g., ["EPIC", "ITEM", "TASK"])
    /// - `file_path_and_name`: Dot-separated comment block path to validate
    ///
    /// # Returns:
    /// - `Ok(())` if the path is valid
    /// - `Err(String)` with descriptive error message if validation fails
    ///
    /// # Error Messages:
    /// - "There is no file path in the first line of the comment block." - Empty path
    /// - "Path is longer than what is allowed." - Path exceeds maximum depth
    /// - "Comment block name must be unique in code base." - Duplicate block name
    /// - "Invalid folder prefix [actual] [expected]." - Path component doesn't match prefix
    ///
    /// # Example:
    /// For folder_prefixes = ["EPIC", "ITEM", "TASK"]:
    /// - Valid: "EPIC epic.ITEM item.TASK task.Description"
    /// - Invalid: "EPIC epic.ITEM wrong.TASK task" (second component doesn't start with "ITEM")
    /// - Invalid: "EPIC epic.ITEM item.TASK task.Too.Long" (exceeds maximum depth)
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
    /// Initializes a new comment block by extracting metadata from the first comment line.
    ///
    /// This function is called when transitioning from CODE to COMMENT state to process
    /// the first line of a comment block, which contains the comment block name and
    /// serves as the metadata header for the entire comment block.
    ///
    /// # Key Operations:
    /// - **Name Extraction**: Strips the comment marker prefix and trims whitespace to get
    ///   the comment block name (e.g., "//# EPIC.Get Lines.ITEM Test Block [1]" → "EPIC.Get Lines.ITEM Test Block [1]")
    /// - **Line Number Recording**: Sets `comment_line_start` to track where the comment block begins
    /// - **Name Storage**: Stores the extracted comment block name for later processing
    ///
    /// # Parameters:
    /// - `line`: The first line of a comment block including the comment marker prefix
    ///
    /// # Returns:
    /// - `Ok(())` on successful initialization
    /// - `Err(String)` if the comment block name is invalid (though currently no validation occurs)
    ///
    /// # Note:
    /// - The comment block name typically follows a hierarchical naming convention with
    ///   dot-separated components (e.g., "EPIC.ITEM.TASK")
    /// - The line number is recorded as `line_counter + 1` because `line_counter` tracks
    ///   the line that was just processed, and we want the starting line of the comment
    /// - This function is called exclusively by `parse_comment` during state transitions
    /// - The extracted comment block name will later be processed by `strip_number_in_str`
    ///   to separate Sequence numbers from the actual block name
    fn parse_comment_start(&mut self, line: &str) -> Result<(), String> {
        let comment_name = line[self.start_of_comment.len()..].trim();
        self.comment_line_start = self.line_counter + 1;
        self.current_comment_name = comment_name.to_string();
        Ok(())
    }
    /// Processes individual comment lines and manages comment block state transitions.
    ///
    /// This function handles the core logic of parsing comment lines and managing the state
    /// machine transitions between CODE and COMMENT states. It distinguishes between the
    /// first line of a comment block (which contains metadata) and subsequent comment lines.
    ///
    /// # State Machine Logic:
    /// - **CODE → COMMENT**: When encountering the first comment line, transitions to COMMENT state
    ///   and calls `parse_comment_start` to extract block metadata (name, line number)
    /// - **COMMENT → COMMENT**: When already in COMMENT state, adds the line content to the
    ///   current comment block buffer
    ///
    /// # Line Processing:
    /// - **First Comment Line**: Contains the comment block name and triggers state transition
    /// - **Subsequent Lines**: Contain actual comment content, stripped of the comment marker
    ///
    /// # Parameters:
    /// - `line`: The raw comment line including the comment marker prefix
    ///
    /// # Returns:
    /// - `Ok(())` on successful parsing
    /// - `Err(String)` if `parse_comment_start` fails (e.g., invalid comment block name)
    ///
    /// # Error Conditions:
    /// - Failure in `parse_comment_start` when processing the first comment line
    /// - Invalid comment block name format
    ///
    /// # Note:
    /// This function is called by `parse_file` for every line that starts with the
    /// comment marker string. It's responsible for the state transitions that define
    /// comment block boundaries.
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
    //#EPIC Get Lines.ITEM Write out all of the history [0]
    //#
    //##Write out all blocks encountered in the past after the last file was processed
    /// Finalizes and stores a completed comment block into the comment history.
    ///
    /// This function is called when a comment block ends (either by encountering non-comment lines
    /// or reaching end of file) to process the accumulated comment lines and store them in the
    /// comment history for later output.
    ///
    /// # Process Flow:
    /// 1. **State Transition**: Returns parser state from COMMENT to CODE
    /// 2. **Block Preparation**: Adds source file metadata and line number to comment block
    /// 3. **Sequence Extraction**: Parses Sequence number from comment block name using regex
    /// 4. **History Storage**: Stores the comment block in the hierarchical comment history
    /// 5. **Duplicate Prevention**: Checks for duplicate Sequence numbers in the same block name
    /// 6. **Cleanup**: Clears the current comment buffer for the next block
    ///
    /// # Key Operations:
    /// - **Metadata Addition**: Prepends source file path and line number to comment block
    /// - **Sequence Management**: Extracts and validates Sequence numbers from block names
    /// - **Hierarchical Storage**: Organizes comments by documentation path and Sequence
    /// - **Duplicate Detection**: Ensures unique Sequence numbers per comment block name
    ///
    /// # Parameters:
    /// - `file_name`: Source file path where the comment block was found
    /// - `doc_root`: Base documentation path for organizing output
    ///
    /// # Returns:
    /// - `Ok(())` on successful storage
    /// - `Err(std::io::Error)` if duplicate Sequence numbers are detected
    ///
    /// # Error Conditions:
    /// - Duplicate Sequence numbers in the same comment block name
    /// - Invalid Sequence number format in comment block name
    ///
    /// # Note:
    /// The function uses BTreeMap to maintain comment blocks in Sequence order and
    /// HashSet to ensure unique comment block names across the entire codebase.
    fn write_out_all_history(
        &mut self,
        file_name: &str,
        doc_root: &str,
    ) -> Result<(), std::io::Error> {
        self.current_state = State::CODE;
        if self.comment.len() > 0 {
            let mut all_block_lines = vec![format!(
                "[SOURCE FILE:](file:///{file_name}) LINE: {}\n",
                self.comment_line_start
            )];
            // keep history of comments
            all_block_lines.append(&mut self.comment);
            let comment_name = self.strip_number_in_str(&self.current_comment_name)?;

            let check_insert = self
                .comment_history
                .entry(format!("{doc_root}.{}", comment_name.1))
                .or_insert_with(|| BTreeMap::new())
                .insert(comment_name.0, all_block_lines);

            if check_insert.is_some() {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!(
                        "Duplicate Sequence number exist in name of block {}",
                        comment_name.0
                    ),
                ));
            }

            self.comment_block_names
                .insert(self.current_comment_name.clone());
            self.comment.clear();
        }
        Ok(())
    }
    //#EPIC Get Lines.ITEM Parse file for line blocks [0]
    //#
    //## Parse file for line blocks
    //#Open the file iff it exist. Read the file line by line and check if the line starts with the _start_
    //#string. If the line does start with the _start_ string then keep the line in the current _comment_
    /// Parses a source file to extract specially formatted comment blocks and organize them into documentation.
    ///
    /// This function implements a state machine that processes files line by line, looking for comment blocks
    /// that start with a specific marker string. It handles the complete life cycle of comment extraction:
    ///
    /// # Process Flow:
    /// 1. **File Setup**: Opens the file and sets up folder prefix hierarchy from the dot-delimited prefix string
    /// 2. **Line Processing**: Reads each line and checks for comment markers
    /// 3. **State Management**: Tracks whether currently in CODE or COMMENT state
    /// 4. **Comment Extraction**: When in COMMENT state, collects lines into comment blocks
    /// 5. **Block Finalization**: Writes out completed comment blocks when returning to CODE state or EOF
    ///
    /// # State Transitions:
    /// - **CODE → COMMENT**: When encountering a line starting with `start_of_comment`
    /// - **COMMENT → CODE**: When encountering a non-comment line while in COMMENT state
    /// - **Any → ERROR**: When parsing errors occur
    ///
    /// # Error Handling:
    /// - I/O errors are propagated via Result
    /// - Parsing errors set ERROR state and log to file/stdout
    /// - Line counter tracks position for error reporting
    ///
    /// # Parameters:
    /// - `file_name`: Path to source file to parse
    /// - `doc_root`: Root directory for generated documentation
    /// - `folder_prefix`: Dot-delimited string defining folder hierarchy for output
    ///
    /// # Returns:
    /// - `Ok(())` on successful parsing
    /// - `Err(std::io::Error)` on I/O or parsing failures
    fn parse_file(
        &mut self,
        file_name: &str,
        doc_root: &str,
        folder_prefix: &'a str,
    ) -> Result<(), std::io::Error> {
        let file = File::open(file_name)?;
        let buf_reader = io::BufReader::new(file);
        let folder_prefixes: Vec<&'a str> = folder_prefix.split(".").collect();
        self.folder_prefixes = folder_prefixes;
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
                        println!("parse file {message}");
                    }
                }
            } else {
                if self.current_state == State::COMMENT {
                    self.write_out_all_history(file_name, doc_root)?;
                }
            }
            self.line_counter += 1u16;
        }
        if self.current_state == State::COMMENT {
            self.write_out_all_history(file_name, doc_root)?;
        }
        Ok(())
    }
    //#EPIC Get Lines.ITEM Get Line Blocks in all files [0]
    //#
    //## Get all the line blocks by looking at all the files in the folder having the file name extension
    //#Get all the files and filter by file type and file extension, then parse the filtered files. Write
    //#out any errors encountered to the console.
    /// Orchestrates the extraction of comment blocks from all files in a directory tree.
    ///
    /// This is the main entry point for the documentation generation system. It recursively scans
    /// a directory structure, processes all files with the specified extension, extracts comment
    /// blocks, and generates organized documentation output.
    ///
    /// # Process Flow:
    /// 1. **Setup**: Clears existing documentation directory and initializes parser state
    /// 2. **Directory Traversal**: Recursively walks through the folder structure using WalkDir
    /// 3. **File Filtering**: Processes only files with the specified extension
    /// 4. **File Processing**: Calls `parse_file` on each matching file to extract comments
    /// 5. **Error Handling**: Logs parsing errors but continues processing other files
    /// 6. **Finalization**: Writes out all accumulated comment history to documentation files
    ///
    /// # Key Features:
    /// - **Recursive Scanning**: Follows symbolic links and processes subdirectories
    /// - **File Type Filtering**: Only processes files with specified extension (e.g., ".rs", ".py")
    /// - **Error Resilience**: Continues processing even when individual files fail
    /// - **Clean Output**: Removes existing documentation before generating new content
    ///
    /// # Parameters:
    /// - `folder_name`: Root directory to scan for source files
    /// - `doc_root`: Output directory for generated documentation
    /// - `start`: String that marks the beginning of comment blocks (e.g., "//#")
    /// - `folder_prefixes`: Dot-delimited hierarchy for organizing output documentation
    /// - `file_extension`: File extension filter (e.g., "rs" for Rust files)
    ///
    /// # Side Effects:
    /// - Removes and recreates the `doc_root` directory
    /// - Creates markdown files in the documentation hierarchy
    /// - Prints error messages to console for failed file processing
    ///
    /// # Note:
    /// This function doesn't return a Result but handles errors internally by logging them,
    /// allowing the process to continue even when individual files fail to parse.
    pub fn comment_in_files(
        &mut self,
        folder_name: &str,
        doc_root: &str,
        start: &str,
        folder_prefixes: &'a str,
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
                    if let Err(error) = self.parse_file(name, doc_root, folder_prefixes) {
                        println!("comment in file {error:?}");
                    } else {
                        if self.current_state == State::ERROR {
                            println!("Error occurred while parsing file: {}", name);
                        }
                        // to do log None case as file is deleted while getting scanned
                    }
                }
            }
        }
        // all files is processed to print out the history of self lines
        if let Err(error) = self.write_history() {
            println!("write history {error:?}");
        };
    }
}

#[cfg(test)]
#[test]
fn test_if_file_path_is_valid() {
    let mut comments = Comments::default();
    let path = &vec!["EPIC", "ITEM", "TEST"];
    if let Err(error) = comments.is_valid_folder_path(path, "EPIC epic.ITEM item.TEST test") {
        println!("test {error}");
    }
    comments.current_comment_name = "EPIC epic.ITEM item.TEST test".to_string();
    comments
        .comment_block_names
        .insert(comments.current_comment_name.clone());
    if let Err(error) = comments.is_valid_folder_path(path, "EPIC epic.ITEM item.TEST test") {
        println!("{error}");
    }
}
