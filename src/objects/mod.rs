use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;

pub use blob::Blob;
pub use commit::Commit;
pub use tree::Tree;
pub use tree::TreeEntry;

mod blob;
mod commit;
mod tree;

//哈希算法为Digest
pub type Hash = Digest;

//Git对象特征
pub trait Object {
    //Git对象转换文件数据方法
    fn dump(&self) -> Vec<u8>;
    
    //文件句柄转换Git对象方法
    fn from<R: BufRead>(reader: R) -> Box<Self>;

    ///生成哈希值方法
    fn hash(&self) -> Hash {
        Sha1::from(self.dump()).digest()
    }

    //加载Git对象方法
    fn load(repo: &PathBuf, hash: Hash) -> Box<Self> {
        //生成对象文件路径名
        //文件路径名："主路径/objects"
        let mut objects_path = repo.join("objects");
        //哈希值进行切片
        //文件路径名："主路径/objects/哈希值前两位"
        objects_path.push(&hash.to_string()[..2]);
        //文件路径名："主路径/objects/哈希值前两位/哈希值剩余位"
        objects_path.push(&hash.to_string()[2..]);
        //ZlibDecoder解码文件内容
        let decoder = ZlibDecoder::new(BufReader::new(
            fs::File::open(&objects_path).expect("Error decoding the object"),
        ));
        //创建Git对象
        Self::from(BufReader::new(decoder))
    }

    //保存Git对象方法
    fn save(&self, repo_path: &PathBuf) {
        //文件内容生成哈希值
        let hash = self.hash().to_string();
        //判断目录"主路径/objects/哈希值前两位"是否存在
        //若不存在则创建目录
        let repo_path = &repo_path.join("objects").join(&hash[..2]);
        if !repo_path.is_dir() {
            fs::create_dir(repo_path).expect("Fail creating object directory");
        }
        //判断文件"主路径/objects/哈希值前两位/哈希值剩余位"是否存在
        //若不存在则创建文件
        let repo_path = repo_path.join(&hash[2..]);
        if !repo_path.is_file() {
            //ZlibEncoder编码文件内容
            let file = File::create(repo_path).expect("Fail opening the object file");
            let mut data = ZlibEncoder::new(file, Compression::default());
            //编码内容写入文件
            data.write_all(&self.dump())
                .expect("Error writing data to the object file");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Blob;
    use super::Tree;
    use super::*;
    use chrono::offset::TimeZone;
    use chrono::FixedOffset;
    use commit::User;
    use std::str::FromStr;

    #[test]
    fn blob_hash() {
        let blob = Blob::new(String::from("Hey").into_bytes());
        assert_eq!(
            blob.hash().to_string(),
            "63cd04a52f5c8cb95686081b000223e968ed74f4"
        );
    }

    #[test]
    fn tree_hash_simple() {
        let mut tree = Tree::new();
        tree.add_file(
            String::from("lol"),
            Hash::from_str("63cd04a52f5c8cb95686081b000223e968ed74f4").unwrap(),
        );
        assert_eq!(
            tree.hash().to_string(),
            "1953c52d154c2ae716190669a376235df8ac1cce"
        );
    }
    #[test]
    fn tree_hash_symlink() {
        let mut tree = Tree::new();
        tree.add_symlink(
            String::from("lol_link"),
            Hash::from_str("21c7de8be9398f4b356ffe7d75838fa166b4d5a6").unwrap(),
        );
        assert_eq!(
            tree.hash().to_string(),
            "828ed76b504d419d56d72df04c1bbb477ea69109"
        );
    }

    #[test]
    fn tree_hash_multiple() {
        let mut tree = Tree::new();
        let mut sub_tree = Tree::new();
        sub_tree.add_file(
            String::from("lol"),
            Hash::from_str("9daeafb9864cf43055ae93beb0afd6c7d144bfa4").unwrap(),
        );
        tree.add_directory(String::from("dir"), sub_tree)
            .add_file(
                String::from("lol"),
                Hash::from_str("9daeafb9864cf43055ae93beb0afd6c7d144bfa4").unwrap(),
            )
            .add_executable(
                String::from("run.sh"),
                Hash::from_str("06206319b8e1c7d41d1b6cd5d7227ec8ef75822d").unwrap(),
            );
        assert_eq!(
            tree.hash().to_string(),
            "6239e26ab616cf842da4555f727a1b1b64d3868a"
        );
    }

    #[test]
    fn commit_hash() {
        let date = FixedOffset::east(7200).timestamp(1561665499, 0);

        let commit = Commit::new(
            Hash::from_str("07f9cb6648d474785a4e08afe408633b1cf04d50").unwrap(),
            vec![Hash::from_str("bed08c07a4fb5d3be29024eac3b7efd7d8729e46").unwrap()],
            User::new(
                String::from("Florian Amsallem"),
                String::from("florian.amsallem@epita.fr"),
            ),
            date,
            String::from("second: commit\n"),
        );
        assert_eq!(
            commit.hash().to_string(),
            "3f07efedb395e8e29412149b5d596f163af24ad4"
        );
    }
}
