# libchai 汉字编码输入方案优化算法

`libchai` 是使用 Rust 实现的汉字编码输入方案的优化算法。它同时发布为一个 Rust crate 和一个 NPM 模块，前者可以在 Rust 项目中安装为依赖来使用，后者可以通过汉字自动拆分系统的图形界面来使用。

`chai` 是使用 `libchai` 实现的命令行程序，用户提供方案的配置文件、词信息文件等，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。

## 使用 `chai`

在[发布页面](https://github.com/hanzi-chai/libchai/releases)根据您的操作系统下载相应的压缩包，支持 Windows, macOS, Linux (GNU), Linux (musl) 等多种不同的环境。压缩包中有以下的示例文件：

- `examples/米十五笔.yaml`: 配置文件示例，具体的格式解释参见 [config.yaml 详解](https://docs.chaifen.app/docs/tutorial/config)；这个文件也可以由[汉字自动拆分系统](https://chaifen.app/)生成；
- `examples/米十五笔.txt`: 词信息文件示例，每个字一行，每行的内容依次为汉字、空格分隔的汉字拆分序列；这个文件也可由自动拆分系统生成；
- `assets/key_distribution.txt`：用指分布文件示例，每个按键一行，每行的内容为以制表符分隔的按键、目标频率、低频率惩罚系数、高频率惩罚系数；
- `assets/pair_equivalence.txt`：双键速度当量文件示例，每个按键组合一行，每行的内容为以制表符分隔的按键组合和当量；

命令行程序基本的用法为：

```bash
./chai [方案文件] -e [词信息文件] [命令]
```

`chai` 支持两个不同的命令：`encode` 和 `optimize`：

- `encode`：使用方案文件和拆分表计算出字词编码并统计各类评测指标
- `optimize`：基于拆分表和方案文件中的配置优化元素布局

例如，您可以运行

```bash
./chai examples/米十五笔.yaml -e examples/米十五笔.txt encode
```

完整的使用说明可用 `./chai --help` 查看。

## 使用 `libchai`

若命令行程序的功能不能满足您的要求，您可以通过编程的方式直接使用 `libchai`。首先在本地配置好 Rust 环境，然后将 `libchai` 安装为依赖。您可以参照 [`libchai-smdc`](https://github.com/hanzi-chai/libchai-smdc) 项目来进一步了解如何通过二次开发来实现个性化的编码、评测、优化逻辑。

## 开发

需要首先运行 `fetch` 脚本下载相关数据资源。然后 `cargo run` 即可编译运行。

您也可以运行 `cargo bench` 来运行性能测试。
