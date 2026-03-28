# Front

简单静态前端，用来可视化 `rust_api` 的 PDF 翻译任务。

## 构建

先安装一次前端依赖：

```bash
cd /home/wxyhgk/tmp/Code/front
npm install
```

然后生成自托管的 Tailwind CSS：

```bash
npm run build:css
```

构建产物会写回 `front/styles.css`，页面运行时不依赖 CDN。

## 运行

先启动 Rust API：

```bash
cd /home/wxyhgk/tmp/Code/rust_api
/home/wxyhgk/tmp/Code/rust_api/target/debug/rust_api
```

再启动一个静态文件服务：

```bash
cd /home/wxyhgk/tmp/Code/front
python -m http.server 8080
```

打开：

```text
http://127.0.0.1:8080
```

默认后端地址是：

```text
http://127.0.0.1:41000
```
