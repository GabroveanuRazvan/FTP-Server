use std::fs;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};



/// Data structure used to manage files in a directory hierarchy.
/// Each file will have a unique name.
#[derive(Debug,Clone)]
pub struct DirectoryTree<T>
    where T: AsRef<Path>
{
    root_dir: T,
}

impl<P> DirectoryTree<P>
where P: AsRef<Path>{

    /// Creates a new tree by recursively creating the missing directories.
    ///
    pub fn new(root_dir: P) -> Result<Self> {

        fs::create_dir_all(&root_dir)?;

        Ok(Self {
            root_dir,
        })
    }

    /// Attempts to create a new tree from an already existing directory.
    ///
    pub fn new_from_existing(root_dir: P) -> Result<Self> {

        if !root_dir.as_ref().exists() {
            let error = Error::new(ErrorKind::NotFound, "Directory does not exist");
            return Err(error);
        }

        Ok(Self {
            root_dir,
        })
    }

    /// Create a directory relative to the tree root.
    ///
    pub fn create_dir<T:AsRef<Path>>(&self, new_dir: T) -> Result<()> {
        let dir_path = self.root_dir.as_ref().join(&new_dir);
        fs::create_dir(dir_path)?;

        Ok(())
    }

    /// Create directories recursively relative to the tree root.
    ///
    pub fn create_dir_all<T:AsRef<Path>>(&self, new_dir: T) -> Result<()> {
        let dir_path = self.root_dir.as_ref().join(&new_dir);
        fs::create_dir_all(dir_path)?;

        Ok(())
    }

    /// Checks if a directory exists in the hierarchy relative to the tree root.
    ///
    pub fn exists_dir<T:AsRef<Path>>(&self, dir: T) -> bool {
        let dir_path = self.root_dir.as_ref().join(&dir);
        dir_path.exists()
    }

    /// Creates a new file into the selected directory if there is no file named the same.
    /// The selected directory is considered to be relative to the tree root.
    pub fn create_file<T:AsRef<Path>>(&self,file_dir: T,file_name: &str) -> Result<()> {

        if let Ok(Some(_)) = self.find_file(file_name){
            let error = Error::new(ErrorKind::AlreadyExists, format!("A file named {} already exists", file_name));
            return Err(error);
        };

        let file_dir_path = self.root_dir.as_ref().join(file_dir.as_ref());

        if !file_dir_path.exists() {
            self.create_dir_all(file_dir.as_ref())?;
        }

        let file_path = file_dir_path.join(&file_name);
        println!("Creating file {:?}", file_path);
        File::create(&file_path)?;


        Ok(())

    }

    /// Recursively searches each child directory of the parent directory and appends to a vec each file path.
    /// Basically a file system DFS.
    pub fn list_files_in_tree(&self) -> Result<Vec<PathBuf>> {

        let dir = self.root_dir.as_ref();

        let mut files: Vec<PathBuf> = Vec::new();

        // Search each child of this directory
        for entry in fs::read_dir(dir)? {
            let entry = entry?;

            let path = entry.path();

            // If the file is a directory go deeper into the file hierarchy or push the file
            if path.is_dir(){

                let dir_node = DirectoryTree::new_from_existing(path)?;
                let files_subset = dir_node.list_files_in_tree()?;
                files.extend(files_subset);

            }else{
                files.push(path);
            }

        }

        Ok(files)

    }

    /// Attempts to find a file and return its path in the directory tree.
    ///
    pub fn find_file(&self, file_name: &str) -> Result<Option<PathBuf>> {
        let dir = self.root_dir.as_ref();

        let result = None;

        for entry in fs::read_dir(dir)? {
            let entry = entry?;

            let path = entry.path();

            if path.is_dir(){

                let dir_node = DirectoryTree::new_from_existing(path)?;
                let result = dir_node.find_file(&file_name)?;

                if result.is_some() {
                    return Ok(result);
                }


            }else{

                let current_file_name = entry.file_name();
                let current_file_name = current_file_name.as_os_str().to_str().unwrap();

                if file_name == current_file_name {

                    return Ok(Some(path));
                }

            }
        }

        Ok(result)

    }

    /// Attempts to find the path of the file in the tree and attempts to remove it.
    pub fn remove_file(&self,file_name: &str) -> Result<()> {

        let file_path = self.find_file(file_name)?;

        let file_path = match file_path {
            Some(file_path) => file_path,
            None => {
                let error = Error::new(ErrorKind::NotFound, "File not found");
                return Err(error);
            },
        };

        fs::remove_file(file_path)?;

        Ok(())

    }

}

#[cfg(test)]
mod tests {
    use std::fs::remove_file;
    use super::*;
    #[test]
    pub fn test_list_files_in_dir_1(){

        let dir = PathBuf::from("./tests/random_dir");
        let dir_tree = DirectoryTree::new_from_existing(dir);

        assert!(dir_tree.is_err());

    }

    #[test]
    pub fn test_list_files_in_dir_2(){

        let dir = PathBuf::from("./tests/files");
        let dir_tree = DirectoryTree::new_from_existing(dir).unwrap();
        let files  = dir_tree.list_files_in_tree();

        assert!(files.is_ok());
        assert_eq!(files.unwrap().len(),3);

    }

    #[test]
    pub fn test_list_files_in_dir_3(){

        let dir = PathBuf::from("./tests/files2");
        let dir_tree = DirectoryTree::new_from_existing(dir).unwrap();
        let files = dir_tree.list_files_in_tree();


        assert!(files.is_ok());
        assert_eq!(files.unwrap().len(),5);


    }

    #[test]
    pub fn test_list_files_in_dir_4(){
        let dir = PathBuf::from("./tests/files");
        let dir_tree = DirectoryTree::new_from_existing(dir).unwrap();

        let found_file = dir_tree.find_file("1").unwrap();
        println!("{:?}", found_file);
        assert!(found_file.is_some());

        let found_file = found_file.unwrap();

        assert!(found_file.is_file());

    }

    #[test]
    pub fn test_list_files_in_dir_5(){
        let dir = PathBuf::from("./tests/files2");
        let dir_tree = DirectoryTree::new_from_existing(dir).unwrap();

        let found_file = dir_tree.find_file("3").unwrap();

        assert!(found_file.is_some());

        let found_file = found_file.unwrap();

        assert!(found_file.is_file());

    }

    #[test]
    pub fn test_list_files_in_dir_6(){
        let dir = PathBuf::from("./tests/files3");
        let dir_tree = DirectoryTree::new_from_existing(dir).unwrap();

        let file_path = PathBuf::from("dir1/dir2/dir3");
        let file_name = "file.txt";

        let result = dir_tree.create_file(file_path, file_name);

        println!("{:?}", result);

        assert!(result.is_ok());

        remove_file("./tests/files3/dir1/dir2/dir3/file.txt").unwrap();

    }

    #[test]
    pub fn test_list_files_in_dir_7(){
        let dir = PathBuf::from("./tests/files4");
        let dir_tree = DirectoryTree::new_from_existing(dir).unwrap();


        let file_name = "file.txt";

        let result = dir_tree.remove_file(file_name);

        println!("{:?}", result);

        assert!(result.is_ok());

        File::create("./tests/files4/file.txt").unwrap();

    }

    #[test]
    pub fn test_list_files_in_dir_8(){
        let dir = PathBuf::from("./tests/files5");
        let dir_tree = DirectoryTree::new_from_existing(dir).unwrap();


        assert!(dir_tree.exists_dir("dir"))

    }

}