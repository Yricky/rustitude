fn main() {
    prost_build::Config::new()
        .out_dir("src/pb") //设置proto输出目录
        .compile_protos(
            &["proto/vector_tile.proto", "proto/geobuf.proto"],
            &["./proto/"],
        ) //我们要处理的proto文件
        .unwrap();
}
