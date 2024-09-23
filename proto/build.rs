fn main() {
    tonic_build::configure()
        .build_server(true)  // 生成服务器端代码
        .build_client(true)  // 生成客户端代码
        .out_dir("src/")  // 生成文件存放位置
        .compile(&["proto/runnel.proto"], &["proto"])
        .unwrap();
}
