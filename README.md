# Ghost Hunter 排行榜

## 请求示例

```
POST http://127.0.0.1:9000/ HTTP/1.1
Content-Type: application/json

{
    "team": "committee",
    "score": -100000,
    "time": "2024-10-20 18:46:14+08:00"
    // date --rfc-3339=secondsz
}
###

GET http://127.0.0.1:9000/ HTTP/1.1
```

如果使用 curl 发送 POST, 应当这样写

```sh
curl --location 'http://127.0.0.1:9000/' \
--header 'Content-Type: application/json' \
--data '{
    "team": "committee",
    "score": -200,
    "time": "2024-10-21 02:05:14+08:00"
}'
```
