# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此仓库中工作提供指导。

## 项目简介

libchai 是一个用于优化汉字编码输入方案的 Rust 库和命令行工具。它根据配置生成字词编码、评估性能指标，并通过模拟退火算法寻找最优的元素-按键分配方案。

项目同时以 Rust crate 和 WebAssembly/NPM 模块两种形式发布。

## 常用命令

```bash
# 构建
cargo build --release

# 构建 WASM 版本
make wasm

# 运行命令行工具
cargo run -- encode    # 计算编码和指标
cargo run -- optimize  # 优化元素布局
cargo run -- server    # 启动 HTTP API 服务（端口 3200）

# 基准测试（使用 criterion；项目无单元测试）
cargo bench
cargo bench -- "四码定长字词"   # 运行特定基准测试
```

## 架构

### 数据流

1. 用户提供 YAML 方案配置（如 `examples/米十五笔.yaml`）和字词频率数据
2. **Config**（`config.rs`）解析方案配置
3. **Context**（`contexts/default.rs`）加载配置并将数据预处理为决策空间（棱镜、安排）
4. **Encoder**（`encoders/default.rs`）根据当前安排生成编码序列
5. **Objective**（`objectives/`）通过指标和缓存评估解的质量
6. **Operator**（`operators/default.rs`）对安排提出变异候选
7. **Optimizer**（`optimizers/simulated_annealing.rs`）驱动搜索循环
8. **Interface**（`interfaces/`）提供命令行、HTTP API 或 WebAssembly 入口

### 核心类型

- `安排`（Arrangement）— 元素到按键的当前映射，即决策变量
- `棱镜`（Prism）— 元素、按键与索引之间的双向映射
- `可编码对象`（Encodable）— 带有元素序列和频率的字或词
- `编码信息` — 编码结果（全码、简码、排名）

### 模块说明

| 路径 | 作用 |
|---|---|
| `src/config.rs` | YAML 配置解析与方案定义 |
| `src/contexts/` | Context trait 及默认实现；持有整体编码状态 |
| `src/encoders/` | 将安排转换为具体编码表 |
| `src/objectives/` | 目标函数、指标计算与结果缓存 |
| `src/operators/` | 生成候选安排的变异算子 |
| `src/optimizers/` | 模拟退火（主要）和遗传算法（备用） |
| `src/interfaces/` | 命令行（`command_line.rs`）、HTTP 服务（`server.rs`）、Web（`web.rs`） |
| `src/server.rs` | 基于 Axum 的 HTTP 服务（由 `server` feature 控制） |
| `src/main.rs` | 二进制入口，需启用 `server` feature |
| `src/lib.rs` | 库根，导出所有公共模块 |
| `benches/benchmark.rs` | 编码性能的 Criterion 基准测试 |

### Feature 标志

- `server` — 启用 HTTP API 服务（Axum/Tokio）；构建 `chai` 二进制时必须开启
- 默认构建目标为 WASM（`cdylib`）和 Rust 库（`rlib`）；`make wasm` 调用 `wasm-pack`

### 配置文件

方案配置为 YAML 文件，可参考 `examples/米十五笔.yaml` 和 `examples/冰雪四拼.yaml`。`assets/` 目录包含评估时使用的按键分布和等价表。
