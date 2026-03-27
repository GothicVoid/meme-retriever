-- 将 ocr_fts 改用 trigram tokenizer，解决中文子串搜索问题。
--
-- 默认的 unicode61 tokenizer 把整段中文当作单个 token，导致只能匹配文字开头
-- （前缀通配符 query*），无法匹配"操作"在"还有这种操作"中间的位置。
--
-- trigram tokenizer 将内容拆成三字符序列（n-gram），任意 MATCH 'keyword'
-- 等价于 LIKE '%keyword%'；不足 3 字符的查询回退线性扫描，全部情况均正确。
--
-- 重建后从 ocr_texts 回填数据（ocr_texts 是内容真实存储，ocr_fts 只是索引）。

DROP TABLE IF EXISTS ocr_fts;

CREATE VIRTUAL TABLE ocr_fts
    USING fts5(image_id UNINDEXED, content, tokenize = 'trigram');

INSERT INTO ocr_fts(image_id, content)
    SELECT image_id, content FROM ocr_texts;
