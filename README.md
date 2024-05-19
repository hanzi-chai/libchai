# chai: 汉字自动拆分系统［命令行版］

`chai` 是一个使用 Rust 编写的命令行程序。用户提供拆分表以及方案配置文件，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。

## 使用

压缩包解压后，根目录中有几个不同的二进制文件：

- `chai` 是 macOS 系统上的可执行文件，它是一个通用二进制文件，意味着 x86_64 架构和 arm64 架构的 Mac 电脑都能使用；
- `chai.exe` 是 Windows 系统上的可执行文件（基于 MinGW）；
- `chai-musl` 和 `chai-gnu` 是 Linux 系统上的可执行文件，分别基于 musl libc 和 glibc
  - 使用 musl libc 的二进制通常兼容性较好
  - 使用 glibc 的二进制要依赖于运行环境中的 glibc，但是通常运行效率较高

请根据您的运行环境选用适当的二进制文件。

### 输入格式解释及示例

压缩包中有以下的示例文件：

- `config.yaml`: 方案文件（米十五笔），具体的格式解释参见 [config.md](https://github.com/hanzi-chai/docs/blob/main/docs/tutorial/config.md)；这个文件也可以由[汉字自动拆分系统](https://chaifen.app/)生成；
- `elements.txt`: 拆分表文件（米十五笔），每个字一行，每行的内容依次为汉字、制表符和以空格分隔的汉字拆分序列；这个文件也可由自动拆分系统生成；
- `assets/frequency.txt`：词频文件，每个字一行，每行的内容为以制表符分隔的词和词频；
- `assets/key_distribution.txt`：用指分布文件，每个按键一行，每行的内容为以制表符分隔的按键、目标频率、低频率惩罚系数、高频率惩罚系数；
- `assets/pair_equivalence.txt`：双键速度当量文件，每个按键组合一行，每行的内容为以制表符分隔的按键组合和当量；

可执行文件支持三个不同的命令：`encode`, `evaluate` 和 `optimize`，例如

- `encode`：将使用方案文件和拆分表计算出字词编码
- `evaluate`：统计各类评测指标
- `optimize`：将基于拆分表和方案文件中的配置优化元素布局

另外，如果方案文件和拆分表文件的路径不为以上的默认值，可以通过命令行参数提供，例如

```bash
./chai yima.yaml -e yima.txt optimize
```

完整的使用说明可用 `./chai --help` 查看。

## 开发

需要首先运行 `make assets` 下载相关数据资源。然后 `cargo run` 即可编译运行。

## 构建和部署

在任何平台上只需要 `make build` 或者 `cargo build` 即可编译。

在 `.cargo/config` 中有一个 `target.x86_64-pc-windows-gnu` 目标，是给 macOS 交叉编译 Windows 可执行文件用的，如果不做交叉编译或者不是为 Windows 平台编译的话可以忽略。

`make package` 命令在 macOS 上运行的时候可以同时编译当前平台（x86_64 或 arm64）以及 Windows 的可执行文件，并打包为一个 zip 压缩文件，便于发布。
