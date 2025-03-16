#!/bin/bash

# 创建目录
mkdir -p examples
mkdir -p assets

# 下载 assets 目录下的文件
for file in key_distribution.txt pair_equivalence.txt; do
    curl "https://assets.chaifen.app/$file" -o "assets/$file"
done

# 下载 examples 目录下的文件
for file in 冰雪四拼.yaml 冰雪四拼.txt 米十五笔.yaml 米十五笔.txt; do
    curl "https://assets.chaifen.app/$file" -o "examples/$file"
done
