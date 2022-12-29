use crate::objects::Object;
use std::io::BufRead;

//Blob类型
pub struct Blob {
    //文件内容
    pub data: Vec<u8>,
}
//Blob方法
impl Blob {
    //构造方法
    pub fn new(data: Vec<u8>) -> Blob {
        Blob { data }
    }
}
//Blob对象实现Git对象特征
impl Object for Blob {
    //Git对象转换文件数据方法
    fn dump(&self) -> Vec<u8> {
        let header = format!("blob {}\0", self.data.len());
        let mut res = vec![];
        res.reserve(self.data.len() + header.len());
        res.append(&mut header.into_bytes());
        res.append(&mut self.data.clone());
        res
    }

    //文件句柄转换Git对象方法
    fn from<R: BufRead>(mut reader: R) -> Box<Blob> {
        let mut buff = vec![];
        reader.read_until(0, &mut buff).unwrap();
        assert!(std::str::from_utf8(&buff[..5])
            .unwrap()
            .starts_with("blob "));
        buff.clear();
        reader.read_to_end(&mut buff).unwrap();
        Box::new(Blob::new(buff))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_dump() {
        let blob = Blob::new(String::from("Hey").into_bytes());
        let dump = blob.dump();
        assert_eq!(dump.len(), 10);
        assert_eq!(std::str::from_utf8(&dump).unwrap(), "blob 3\0Hey");
    }
}
