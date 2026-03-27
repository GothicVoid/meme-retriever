## 得分公式

### 新框架

```
Final_Score = α·Relevance + β·Popularity
```

建议权重配比：**0.75 : 0.25**  
（相关性是核心，热度作为辅助排序信号）

---

## 一、相关性得分 Relevance（权重 0.75）

保持之前的多路融合逻辑：

```python
Relevance = max(
    w1 · S_keyword,      # 关键词匹配
    w2 · S_ocr,          # 图内文字匹配
    w3 · S_clip          # 语义向量
)
```

### 各路径得分计算

#### S_keyword（实体匹配）
```
完全匹配 → 1.0
部分匹配 → 0.8
关联标签 → 0.5
```

#### S_ocr（文字匹配）
```
S_ocr = 字符覆盖率 × 位置权重
覆盖率 = len(query ∩ ocr_text) / len(query)
```

#### S_clip（语义相似度）
```
S_clip = (cosine_similarity + 1) / 2  # 归一化到[0,1]
```

### 三路权重配置
```
默认 w1 = 0.3, w2 = 0.4, w3 = 0.3

支持在设置页面手动设置，设置完下次查询开始生效
```

---

## 二、热度得分 Popularity（权重 0.25）

```python
Popularity = 使用频率
```

### 2.1 使用频率
```python
freq_score = log(1 + click_count) / log(1 + max_click)
```
对数化避免头部图片过度集中

---

## 三、边界情况处理

### 冷启动（新上传的图）
```python
if meme.use_count == 0:
    popularity = 0.5  # 给予中等初始值
```

### 相关性过低时
```python
if relevance < 0.2:
    return 0  # 直接过滤，不展示
```

