# recfs

Mount 科大睿客网云盘 rec.ustc.edu.cn/recdrive

## 进度

目前还处于 Alpha 阶段

已实现的功能：

- ls
    - 可以列出文件列表，但是用了 dummy attrs

## 实现笔记

框架是 `fuse_mt`，类似于 High-level FUSE API，但其实是一个 Rust 重写的版本而不是 API binding

rec.ustc.edu.cn authentication 用得是 token，位于 header `x-auth-token` field，TTL 似乎较小（应该 < 1d）。
但其认证支持科大统一身份认证，而统一身份认证是 CAS，所以应该是可以自动更新 token 的，目前没实现。

## License

MIT
