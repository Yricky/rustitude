pub mod geobuf;
pub mod vector_tile;

mod test {
    use prost::Message;
    use std::fs;

    use super::{
        geobuf::Data,
        vector_tile::{tile::Layer, Tile},
    };

    #[test]
    fn test_vts() {
        let buf = fs::read("/Users/yricky/Downloads/bing_tile.mvt").unwrap();
        // [1,2,3,4,99].into_iter().for_each(|i|{
        //     buf[i] = !buf[i];
        // });

        // let reader = Reader::new(buf).unwrap();

        let tile = Tile::decode(buf.as_slice()).unwrap();
    }
}
