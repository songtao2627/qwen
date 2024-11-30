# Qwen TTS WebSocket Client

这是一个使用 Rust 编写的阿里云 Qwen TTS (Text-to-Speech) WebSocket 客户端。该程序可以将文本转换为语音，并保存为 MP3 文件。

## 功能特性

- 支持实时文本转语音
- WebSocket 双工通信
- 支持自定义语音参数（音量、语速、音调等）
- 自动保存生成的音频文件

## 环境要求

- Rust 1.56.0 或更高版本
- Cargo 包管理器
- 有效的阿里云 DashScope API Key

## 安装

1. 克隆仓库：

```bash
git clone https://github.com/yourusername/qwen-tts-client.git
cd qwen-tts-client
```

2. 设置环境变量：
```bash
export DASHSCOPE_API_KEY="your_api_key_here"
```

3. 构建项目：
```bash
cargo build --release
```

## 使用方法

1. 运行程序：
```bash
cargo run
```

2. 程序会自动：
   - 连接到阿里云 WebSocket 服务器
   - 发送文本内容（默认是李白的《静夜思》）
   - 生成语音文件并保存为 output.mp3

## 配置选项
根据 [CosyVoice API 参考文档](https://help.aliyun.com/zh/model-studio/developer-reference/cosyvoice-api-reference)

在 `main.rs` 中，您可以修改以下参数：
```rust
// 语音合成参数
"parameters": {
"text_type": "PlainText", // 文本类型
"voice": "longxiaochun", // 发音人
"format": "mp3", // 输出格式
"sample_rate": 22050, // 采样率
"volume": 50, // 音量 (0-100)
"rate": 1, // 语速 (0.5-2.0)
"pitch": 1 // 音调 (0.5-2.0)
}
```

## 错误处理

程序包含完整的错误处理机制：
- API Key 验证错误
- WebSocket 连接错误
- 文件操作错误
- 服务器响应错误

## 依赖项

- tokio：异步运行时
- tokio-tungstenite：WebSocket 客户端
- serde_json：JSON 处理
- uuid：生成唯一任务 ID
- futures-util：异步工具
- native-tls：TLS 支持

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！

## 注意事项

- 请确保 API Key 的安全性，不要将其提交到版本控制系统
- 确保网络连接稳定
- 检查输出目录的写入权限

## 常见问题

1. 401 错误：
   - 检查 API Key 是否正确设置
   - 确认 API Key 是否有效

2. 连接错误：
   - 检查网络连接
   - 确认防火墙设置

3. 文件写入错误：
   - 检查目录权限
   - 确保磁盘空间充足

## 更新日志

### v0.1.0
- 初始版本
- 支持基本的文本转语音功能
- 实现 WebSocket 双工通信