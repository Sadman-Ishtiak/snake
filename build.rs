fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/snake.ico");
        res.set("ProductName", "Snake");
        res.set("FileDescription", "Snake");
        res.set("OriginalFilename", "snake.exe");
        res.compile().expect("failed to compile Windows resources");
    }
}
