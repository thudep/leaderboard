# Ghost Hunter 排行榜

## 请求示例

如果使用 curl 发送 POST, 应当这样写

```sh
curl --location 'http://127.0.0.1:9000/' \
--header 'Content-Type: application/json' \
--data '{
    "team": "committee",
    "score": -200,
    "time": "2024-10-21 02:05:14+08:00",
    "secret": "yoursecret"
}'
```
