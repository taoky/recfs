# recfs

科大睿客网 (rec) 云盘的灵车 FUSE 实现。 

Forked from [myl7/recfs](https://github.com/myl7/recfs). [Old README](./README_old.md). 请勿用于生产环境。

因本程序导致的数据丢失或损坏开发者不负任何责任：

> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
>
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
>
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.

## 实现的文件系统功能

目前还处于 Alpha 阶段

已实现的功能（对应函数可能不符合 POSIX 语义）：

- getattr: 获取文件或文件夹的信息
- opendir: 打开文件夹
- readdir: 读取文件夹文件列表
- statfs: 读取可用空间与总空间信息
- create: 在本地缓存创建新文件
- open: 打开远程的文件，或在本地缓存创建新文件
- read: 读取本地缓存的文件，若文件无本地缓存则下载
- write: 写入数据至本地缓存
- mkdir: 创建文件夹
- unlink: 移动文件至回收站
- rmdir: 移动文件夹至回收站
- rename: 不更名移动文件或文件夹至其他文件夹下，或原地更名（扩展名不变）
- link: 服务端复制文件（不是创建硬链接）
- release: 如果是新建的文件，上传至服务器

目前的程序限制：

- 写入：仅支持创建新文件写入；修改已有文件内容的行为未定义；由于接口限制，无法新建 0 bytes 的文件。
- 回收站（`?Recycle`）仅支持查看文件夹内容。`rm` 删除操作的行为是将文件移动至回收站。
- 备份文件夹（`?Backup`）的行为未测试。
- 由于操作系统限制，`link()`/`ln` 无法发送复制文件夹的命令。
- 程序不会清理临时文件夹的内容。
- 进行读取操作时会先从远程下载完整的文件至临时文件夹，然后再响应读取请求。

## 实现笔记

框架是 `fuse_mt`，类似于 High-level FUSE API，但其实是一个 Rust 重写的版本而不是 API binding。`fuse_mt` 内部维护了 inode 列表。在打开文件/文件夹的时候，用户实现的函数会返回对应的 file handle (`fh`)，这个 file handle 可以在接下来的操作中被其他的文件操作函数用到。所有可以实现的文件系统函数参照 <https://docs.rs/fuse_mt/latest/fuse_mt/trait.FilesystemMT.html>。本项目的文件系统函数实现参考 [fs.rs](src/fs.rs)。

程序假设只有它一个写者，好处是可以省下很多文件夹列举时的开销。Rec 的 API 会返回文件夹（节点）下每个文件的信息。原始的 recfs 实现甚至每次 `getattr()` 都会去 list 一次，结果 `ls` 的开销的 HTTP 请求数是 O(n) 的，特别卡。

[fidmap.rs](src/fidmap.rs) 是结构树缓存。`FidMap` 的作用注释很清楚了：

```rust
pub struct FidMap {
    fhmap: BiBTreeMap<u64, Fid>, // a bidirectional map of "file handle" and Fid
    listing_map: HashMap<Fid, FidCachedList>, // a map from Fid to the HTTP cache of listing
    parent_map: HashMap<Fid, Option<Fid>>, // a map from Fid to its parent
}
```

[cache.rs](src/cache.rs) 是文件缓存，能够处理从远程获取缓存到本地的文件和新创建的文件。写入文件的部分，目前只处理了添加新文件的逻辑，修改已有文件的逻辑很麻烦（感觉肯定很难写好而且很大可能会丢数据），并且和 Rec 的 API 不搭：没有原地更新文件的 API（其实对象存储都是这样的？）。

rec.ustc.edu.cn authentication 用得是 token，位于 header `x-auth-token` field，TTL 似乎较小（应该 < 1d）。
支持两种登录方式：CAS 登录（此时 CAS 用户名和密码会被发送到 rec 的接口，而非统一身份认证的接口，这是参考 Windows 客户端的实现做的）；第二种是根据浏览器登录后的 cookie 登录（如果从安全性考虑，我更推荐这种方式）。尽管 auth token 的 TTL 很小，但是登录同时也提供了 refresh token，在 get/post 的时候，如果发现返回 status code 为 "401"，那么就用 refresh token 更新 auth token 之后再试一次。

## 总结

别用，因为很可能会出问题。如果要程序化批量处理，参考 [reccli](https://github.com/taoky/reccli) 来做。

## License

MIT
