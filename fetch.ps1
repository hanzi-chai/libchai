# 创建目录
New-Item -ItemType Directory -Force -Path "examples"
New-Item -ItemType Directory -Force -Path "assets"

# 下载 assets 目录下的文件
$assetsFiles = @("key_distribution.txt", "pair_equivalence.txt")
foreach ($file in $assetsFiles) {
    $url = "https://assets.chaifen.app/$file"
    $output = "assets/$file"
    Invoke-WebRequest -Uri $url -OutFile $output
}

# 下载 examples 目录下的文件
$examplesFiles = @("冰雪四拼.yaml", "冰雪四拼.txt", "米十五笔.yaml", "米十五笔.txt")
foreach ($file in $examplesFiles) {
    $url = "https://assets.chaifen.app/$file"
    $output = "examples/$file"
    Invoke-WebRequest -Uri $url -OutFile $output
}