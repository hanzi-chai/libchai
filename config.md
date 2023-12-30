# config.yaml 详解

`config.yaml` 是汉字自动拆分系统所使用的配置文件，原则上可以由网页 App 生成，但是如果只需要使用优化功能、不需要使用自动拆分功能的话，也可以自行编写。下面只列出与优化相关的字段，其余字段可以不提供。

## `form` 编码（必填）

这里填写了编码方案的有关信息。

```yaml
...
form:
  alphabet: qwertyuiopasdfghjklzxcvbnm
  mapping:
    日: a
    月: b
...
```

### `form.alphabet` 字母表（必填）

指定了该方案所使用的全部键位。目前支持的全部键位为：

- 字母键：`abcdefghijklmnopqrstuvwxyz`，共 26 个
- 数字键：`1234567890`，共 10 个
- 符号键：`-=[];',./`，共 9 个
- 空格键，用 `_` 表示

合计共 46 个。不支持其他键位的原因是没有当量数据。但如果你提供了其他键位的自定义当量，这些键位也可以参与计算。

### `form.mapping` 键盘映射（必填）

键盘映射是一个 YAML 字典，将方案中的编码元素（如字根、补码、音码等）映射到键位。在优化中，这个映射作为初始布局使用。

## `encoder`（必填）

这里填写了一些编码的细节内容。

### `encoder.max_length` 最大码长（必填）

最大码长不得大于或等于 6。

### `encoder.select_keys` 选择键列表（选填）

```yaml
...
  select_keys: [_, ;] # 候选字词依次使用哪些选择键上屏。默认为只有空格 "_"
...
```

选择键列表包含一组用于上屏候选栏中字词的按键。默认情况下，选择键只有一个，为空格（`['_']`），如果需要选重或自定义上屏键，则需要设置选择键列表。这个列表的长度至少为 1。

该列表中的第一个值称为「首选键」，后面要用到。

### `encoder.auto_select_length` 顶屏码长（选填）

在编码时，系统会为所有长度小于顶屏码长的编码添加首选键，使得对于码长和当量的统计正确。例如，对于四码定长方案来说，这个为 4；而对于二码顶方案来说，这个为 2。

如果不填，则默认为 0，即所有编码都不自动添加首选键。

### `encoder.auto_select_pattern` 顶屏模式（选填）

```yaml
...
  auto_select_pattern: "^[a-z]{2}$" # 顶屏匹配模式，四二顶等特殊顶功需要写。设置 pattern 之后，length 不再生效
...
```

对于复杂顶功方案来说，可以通过设置顶屏模式来实现更为灵活的顶功模式。如果一个编码串符合顶屏模式，就不添加首选键，反之则添加首选键。顶屏模式的优先级高于顶屏码长，如设置则顶屏码长失效。

### `encoder.short_code_schemes` 简码方式（选填）

简码方式定义了如何从全码生成简码。默认情况下，会依次生成所有小于最大码长的简码。如果需要不同的简码生成方式，需要自定义。例如，对于四码定长方案，默认配置等价于如下写法：

```yaml
...
  short_code_schemes:
    - { prefix: 1 } # 取全码的前一码为简码
    - { prefix: 2 } # 取全码的前两码为简码
    - { prefix: 3 } # 取全码的前三码为简码
...
```

除了指定码长外，还可以配置多重简码，以及简码的特殊选择键（如果不填选择键，则默认是用全局选择键）。

```yaml
...
  short_code_schemes:
    - { prefix: 2, count: 2 } # 取全码的前两码为简码，并且出二简二重
    - { prefix: 1, count: 3, select_keys: ",./" } # 取全码的前一码为简码，并且出一简三重，这个特定码长上的选择键的顺序可以覆盖全局的选择键，但是这些键必须也在全局选择键中至少出现一次
...
```

### `encoder.rules` 组词规则（选填）

这个和 Rime 输入法的配置格式完全一样，无需过多解释。

```yaml
...
  rules:
    - length_equal: 2 # 二字词的编码规则
      formula: "AaAbBaBb"
    - length_equal: 3 # 三字词的编码规则
      formula: "AaBaCaCb"
    - length_in_range: [4, 10] # 四至十字词的编码规则
      formula: "AaBaCaZa"
...
```

如果不填，默认为以上规则。

## `optimization` 优化（必填）

这里填写了所有优化相关的配置，分为优化目标、优化算法和优化约束三部分。

### `optimization.objective` 优化目标（必填）

可以分别对单字全码 `characters_full`、单字简码 `characters_short`、词组全码 `words_full`、词组简码 `words_short` 四部分指定一系列的评价指标以及相应的权重，这几部分的指标的数值乘以对应的权重相加之后，再合起来相加作为整体的优化目标。所以，这一部分的配置形如：

```yaml
...
optimization:
  objective:
    characters_full: ...
    character_short: ...
    words: ...
...
```

在每一部分中，可以使用的指标包括动态（加权平均）指标和静态（无加权的数量）指标。

#### 动态指标

动态指标包括选重率、用指当量、速度当量和简码频率四部分。以单字简码为例，每个动态指标可以指定一个相关的权重：

```yaml
...
    characters_short:
      duplication: 10.0 # 减少单字简码动态选重率
      key_equivalence: 0.1 # 减少单字简码用指
      pair_equivalence: 0.1 # 减少单字简码当量
      levels: # 减少单字四键频率
        - { length: 4, frequency: 1.0 }
...
```

「选重率 `duplication`」优化所有在重码字词中处于第二位及之后的字词所占的比例。

「用指当量 `key_equivalence`」优化不同键位击键的难易程度，在 `assets/key_equivalence.txt` 中每个键的击键困难程度都有一个评分，越难按数字越大。

「速度当量 `pair_equivalence`」优化两个键之间手形变化的难易程度的，在 `assets/pair_equivalence.txt` 中每个键对都有一个评分（来自陈一凡的速度当量测量结果）。

「杏码风格速度当量 `new_pair_equivalence`」按照杏码规则优化两个键之间手形变化的难易程度。
在 `assets/pair_equivalence.txt` 中每个键对都有一个评分，越难按数字越大。
请将 new_pair_equivalence.txt复制为pair_equivalence.txt，来使用杏码的原版数据。
也可以使用内置的陈一凡表+杏码规则进行优化，我没有尝试过。

「杏码风格用指当量 `new_key_equivalence`」按照杏码规则优化不同键位击键的难易程度。
在 `assets/key_equivalence.txt` 中每个键的击键困难程度都有一个评分，越难按数字越大。
按照杏码规则，除了首码以外的按键通过组合当量new_pair_equivalence优化，用指当量只优化首码。
请将 new_key_equivalence.txt复制为key_equivalence.txt，来使用杏码的原版数据。
也可以使用内置的表+杏码规则进行优化，我没有尝试过。

「杏码风格用指当量改 `new_key_equivalence_modified`」按照我自己拍脑门的规则优化不同键位击键的难易程度。
这个规则假定输入的连续性，预测上一字结束时的最后一码，与首码拼合来推测首码的用指当量。
同样的，除了首码以外的按键通过组合当量new_pair_equivalence优化，用指当量只优化首码。
该规则只使用pair_equivalence.txt来预测当量，与key_equivalence.txt中的数据无关。
也可以使用内置的陈一凡表+杏码规则进行优化，我没有尝试过。


「简码频率 `levels`」优化出简后编码长度为某一特定长度的字词所占的比例，例如二级简码。注意，这里的 `length` 是指包含空格的长度，所以对于四码定长方案来说，一级简码是 `length: 2`，二级简码是 `length: 3`，等等。另注意，因为优化时默认总的目标函数是越小越好，所以如果想要增加简码的频率，需要把它们的权重设为负数。

下面举一个例子。若一个输入方案只优化单字简码性能，且设定了「选重率」的权重为 10.0、「用指当量」的权重为 0.1，「速度当量」的权重为 0.1。设当前方案的选重率为 1%，用指当量为 1.8，速度当量为 1.4，则该方案的总目标函数值为

$$
0.01 * 10.0 + 1.8 * 0.1 + 1.4 * 0.1 = 0.1 + 0.18 + 0.14 = 0.42
$$

可以看到，一个指标的权重越高，优化时的重要性就越大。可以根据自己的需要调整权重。

#### 静态指标

静态指标是不考虑频率，单纯计算符合某种条件的东西的数量的指标。静态指标一般分为几个层级来统计，例如单字前 1500 有多少选重、3000 有多少选重等等。在 YAML 文件中，是用「层级 `tiers`」这个字段来表达，形如

```yaml
...
      tiers:
        - { top: 1500, duplication: 1.0 } # 优化单字简码前 1500 静态选重率
        - { duplication: 1.0, levels: [{ length: 4, frequency: 10.0 }] } # 优化单字简码静态选重率以及二简的数量
...
```

每个层级上的 `top` 是指统计范围为按频率排序的前若干个字词。如果没有 `top`，意思就是统计全部范围的数据。

在每个层级上，可以统计当前层级的「静态选重率 `duplication`」，以及不同级别的简码的数量。这两部分与之前的动态指标的用法类似，不过多赘述。

### `metaheuristic` 优化算法（必填）

优化算法中需要指定使用的算法种类（目前支持退火算法 `SimulatedAnnealing`）。以下主要介绍退火算法：

```yaml
...
  metaheuristic:
    algorithm: SimulatedAnnealing
    # runtime: 10
    parameters:
      t_max: 0.1
      t_min: 0.00001
      steps: 100000
    report_after: 0.9
    search_method:
      random_move: 0.9
      random_swap: 0.09
      random_full_key_swap: 0.01
...
```

#### `metaheuristic.parameters` 参数（选填）

`parameters` 是指退火的参数，系统将从最高温 `t_max` 开始逐渐下降到最低温 `t_min`，总步数为 `steps`。

#### `metaheuristic.runtime` 运行时间（选填）

如果不填写 `parameters`，但是填写了 `runtime`，系统会根据一定的算法来自动寻找参数，并且根据所提供的时间长度来决定优化步数。`runtime` 的单位是分钟，例如 `runtime: 10` 就是运行 10 分钟。如果 `runtime` 也不填，则默认为 10 分钟。

目前系统如果只计算单字指标，每步的运行时间约为 70 \~ 80 μs；如果同时计算字词指标，每步的运行时间约为 600 \~ 700 μs（以上数据在 Apple Silicon M2 Max 的 Mac Studio 上测得，不同电脑可能有差异）。每小时可以运行六百万步字词的优化，或五千万步单字的优化。

#### `metaheuristic.report_after` 结果汇报（选填）

当优化进度达到这一数值之后，每个更好的方案都会被保存到 `output/` 文件夹下；如果没达到这一数值，就不保存。默认为 0.9。

#### `metaheuristic.search_method` 搜索方法（选填）

系统能够随机移动一个元素的按键，或者随机交换两个元素对应的按键。在搜索方法中，可以自定义这两者之间的比例。默认为 90% 移动，10% 交换。

### `optimization.constraints` 优化约束（选填）

约束是指在优化过程中不能违反的规则，例如某些字根必须在某些键位等。本系统的约束非常灵活，分为 4 大类 7 小类。

#### 固定某个元素 `constraints.elements`

这是最常用的约束，例如形码固定某种补码的键位不变：

```yaml
...
  constraints:
    elements:
      - { element: '11' } # 固定元素 "11" 不变，下同
      - { element: '12' }
      - { element: '13' }
...
```

#### 固定某一码 `constraints.indices`

某些双编码方案可能希望所有小码都是有理的。注意，程序中的下标以 0 为开始，所以第二码不变应该写作 `index: 1`。

```yaml
...
    indices:
      - { index: 1 } # 固定所有元素的第二码不变
...
```

#### 固定某个元素的某一码 `constraints.element_indices`

以上两种的组合

```yaml
...
    element_indices:
      - { element: 日, index: 0 } # 固定字根「日」的第一码不变
...
```

#### 固定约束改为窄化约束

有的时候，我们可能希望一个元素并不是完全固定在某个键位上，而是在几个有限的键位上都可以。比如，让「木」字根在 s, d, f 键上皆可，相应的写法就要改为

```yaml
...
      - { element: 木, keys: [s, d, f] }
...
```

#### 绑定某个元素和另外一个元素的某几码 `constraints.grouping`

例如，想让「土」和「士」字根的大码相同，但小码可以不相同，可以写作

```yaml
...
    grouping:
      - [{ element: 土, index: 0 }, { element: 士, index: 0 }]
...
```
