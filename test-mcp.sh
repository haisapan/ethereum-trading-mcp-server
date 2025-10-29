#!/bin/bash

# 测试 MCP 服务器的简单脚本
# 这个脚本展示如何向 MCP 服务器发送 JSON-RPC 请求

echo "============================================"
echo "🧪 测试 Ethereum Trading MCP Server"
echo "============================================"
echo ""

# 启动服务器（在后台）
echo "📡 启动 MCP 服务器..."
./target/release/ethereum-trading-mcp-server &
SERVER_PID=$!
sleep 2

echo "✅ 服务器已启动 (PID: $SERVER_PID)"
echo ""

# 清理函数
cleanup() {
    echo ""
    echo "🛑 停止服务器..."
    kill $SERVER_PID 2>/dev/null
    echo "✅ 测试完成"
}

# 设置清理陷阱
trap cleanup EXIT

# 示例 1: 初始化请求
echo "📨 发送初始化请求..."
echo '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "test-client",
      "version": "1.0.0"
    }
  }
}' | ./target/release/ethereum-trading-mcp-server

echo ""
echo "============================================"
echo "💡 提示："
echo "   MCP 服务器使用 stdio 传输协议"
echo "   在生产环境中，应该通过 MCP 客户端（如 Claude Desktop）连接"
echo "============================================"
