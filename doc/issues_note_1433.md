## User

https://github.com/calimero-network/core/issues/1433 梳理一下这个issues 要做的事情

## Assistant

根据这个GitHub issue，我来梳理一下需要做的事情：

## Issue分析：改进jsonrpc/admin服务的错误处理

### 问题描述：
当前的错误信息不够具体和有用，只显示了HTTP状态码（如500内部服务器错误、401未授权），而没有提供足够的上下文信息来帮助用户理解具体发生了什么问题。

### 具体问题：
1. **错误信息过于简单**：只显示HTTP状态码，缺乏详细的错误原因
   - 例如：`Request failed with status: 500 Internal Server Error`
   - 例如：`Request failed with status: 401 Unauthorized`

2. **不同输出格式的问题**：
   - 普通输出格式显示`Request failed with status: 500 Internal Server Error`
   - JSON输出格式显示`{"Other":["Request failed with status: 401 Unauthorized"]}`

### 需要改进的地方：

1. **增强错误消息**：
   - 提供更具体的错误原因而不仅仅是HTTP状态码
   - 包含可能的解决方案或建议

2. **统一错误处理**：
   - 确保不同输出格式（普通文本vs JSON）的错误处理一致性
   - 标准化错误消息的结构

3. **提供更多上下文**：
   - 对于认证错误（401），说明是什么原因导致认证失败
   - 对于服务器错误（500），提供更多关于具体失败的信息

4. **改进用户体验**：
   - 让错误消息对用户更有帮助
   - 提供可操作的错误信息

### 涉及的组件：
- `meroctl` CLI工具
- jsonrpc/admin服务
- 错误处理机制
- 客户端连接逻辑（`/crates/client/src/connection.rs`）

这个issue的目标是让错误信息更加用户友好和实用，帮助用户更容易理解和解决遇到的问题。

## User

再继续梳理一下涉及到的代码部分


## Assistant

现在我了解了整个错误处理流程！让我总结一下涉及的代码部分：

## 涉及的代码部分梳理

基于issue和代码分析，我梳理出以下需要改进错误处理的代码部分：

### 1. **主要错误处理流程** (`meroctl`)

**核心文件：**
- `core/crates/meroctl/src/connection.rs` (181-189行) - HTTP请求错误处理的核心位置
- `core/crates/meroctl/src/errors.rs` - `ClientError`定义
- `core/crates/meroctl/src/cli.rs` (145-149, 234-237行) - 错误转换和显示逻辑

**错误流程：**
```core/crates/meroctl/src/connection.rs#L181-189
if !response.status().is_success() {
    bail!(ClientError {
        status_code: response.status().as_u16(),
        message: response
            .text()
            .await
            .map_err(|e| eyre!("Failed to get response text: {e}"))?,
    });
}
```

### 2. **服务端错误处理** (admin API)

**核心文件：**
- `core/crates/server/src/admin/service.rs` (393-426行) - `ApiError`定义和错误解析
- `core/crates/server/src/admin/handlers/applications/install_application.rs` (19-21行) - 应用安装错误处理
- `core/crates/server/src/admin/handlers/context/create_context.rs` (25-27行) - 上下文创建错误处理

**问题：**
```core/crates/server/src/admin/handlers/applications/install_application.rs#L19-21
Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
```

### 3. **认证错误处理**

**核心文件：**
- `core/crates/auth/src/auth/middleware.rs` (85-105行) - JWT认证失败处理
- `core/crates/meroctl/src/connection.rs` (104-172行) - 客户端认证重试逻辑

**现有的认证错误处理：**
```core/crates/meroctl/src/connection.rs#L164-172
if response.status() == 401 && self.jwt_tokens.lock().unwrap().is_some() {
    // Token过期重试逻辑
}

if response.status() == 403 {
    bail!("Access denied. Your authentication may not have sufficient permissions.");
}
```

### 4. **输出格式处理**

**核心文件：**
- `core/crates/meroctl/src/output.rs` (35-47行) - JSON vs Human输出格式切换
- `core/crates/meroctl/src/cli.rs` (230-241行) - 错误报告格式化

**问题分析：**
```core/crates/meroctl/src/cli.rs#L234-237
CliError::ClientError(e) => format!("Client Error ({}): {}", e.status_code, e.message),
```

### 5. **需要改进的具体问题**

1. **错误信息过于泛化：**
   - 当前只显示HTTP状态码和原始错误文本
   - 缺少针对不同错误类型的具体上下文信息

2. **JSON输出格式不一致：**
   - 现在显示`{"Other":["Request failed with status: 401 Unauthorized"]}`
   - 应该有更结构化的错误格式

3. **服务端错误信息不够详细：**
   - 大部分handler只是返回`err.to_string()`
   - 没有根据错误类型提供具体的解决建议

4. **认证错误处理可以改进：**
   - 401错误可以提供更具体的失败原因
   - 可以给出重新认证的具体步骤

### 6. **改进方向**

1. **增强`ClientError`结构：**
   - 添加错误类型分类
   - 包含建议的解决方案
   - 提供更多上下文信息

2. **统一JSON错误格式：**
   - 创建标准化的错误响应结构
   - 确保不同输出格式的一致性

3. **改进服务端错误处理：**
   - 在admin handlers中提供更具体的错误信息
   - 根据不同的错误类型返回相应的HTTP状态码

4. **增强认证错误信息：**
   - 提供更详细的认证失败原因
   - 给出具体的解决建议

这个issue的核心是要让错误信息从简单的HTTP状态码变成对用户有实际帮助的、可操作的错误消息。

## User

给一下 . 改进方向的处理，先不要去修改代码，给我返回一下改进的地方

**Tool Call: Thinking**
Status: Completed

用户要求我给出改进方向的详细处理方案，但不要修改代码，只是列出需要改进的地方。基于前面的代码分析，我需要整理出具体的改进点和方案。

让我按照之前分析的几个方向来详细规划：

1. 增强ClientError结构
2. 统一JSON错误格式
3. 改进服务端错误处理
4. 增强认证错误信息

我需要为每个方向提供具体的改进建议，包括：
- 当前的问题
- 改进的目标
- 具体的实现思路
- 涉及的文件和函数

## Assistant

基于前面的代码分析，我来详细列出改进方向和具体的处理方案：

## 1. 增强`ClientError`结构

### 当前问题：
- `ClientError`只有`status_code`和`message`两个字段
- 错误信息过于简单，如`"Request failed with status: 500 Internal Server Error"`
- 缺少错误分类和建议的解决方案

### 改进方案：
**文件：** `core/crates/meroctl/src/errors.rs`

**需要添加的字段：**
```rust
pub struct ClientError {
    pub status_code: u16,
    pub message: String,
    pub error_type: ErrorType,        // 新增：错误类型分类
    pub suggestion: Option<String>,   // 新增：解决建议
    pub context: Option<String>,      // 新增：错误上下文
    pub details: Option<ErrorDetails>, // 新增：详细信息
}

pub enum ErrorType {
    Authentication,
    Authorization,
    NotFound,
    ServerError,
    NetworkError,
    ValidationError,
    Unknown,
}

pub struct ErrorDetails {
    pub operation: String,           // 正在执行的操作
    pub resource: Option<String>,    // 相关资源
    pub retry_possible: bool,        // 是否可重试
}
```

## 2. 统一JSON错误格式

### 当前问题：
- JSON输出显示`{"Other":["Request failed with status: 401 Unauthorized"]}`
- 结构不统一，难以解析
- Human格式和JSON格式差异很大

### 改进方案：
**文件：**
- `core/crates/meroctl/src/cli.rs` (CliError的Serialize实现)
- `core/crates/meroctl/src/output.rs` (Output格式化)

**统一的错误结构：**
```rust
// JSON输出格式
{
    "error": {
        "type": "authentication",
        "status_code": 401,
        "message": "Authentication required",
        "suggestion": "Please re-authenticate using 'meroctl node add <node>' or check your credentials",
        "context": "Installing application kv_store.wasm",
        "details": {
            "operation": "app install",
            "retry_possible": true
        }
    }
}
```

## 3. 改进服务端错误处理

### 当前问题：
- Admin handlers简单返回`err.to_string()`
- 缺少针对性的错误信息
- 没有根据业务逻辑提供具体错误原因

### 改进方案：

#### A. 应用安装错误 (`install_application.rs`)
**当前代码位置：** `core/crates/server/src/admin/handlers/applications/install_application.rs:19-21`

**需要改进的错误处理：**
- WASM文件格式错误
- 网络下载失败
- 应用已存在
- 权限不足
- 存储空间不足

**建议的错误分类：**
```rust
match err {
    ApplicationError::InvalidWasm => ApiError {
        status_code: StatusCode::BAD_REQUEST,
        message: "Invalid WASM file format. Please ensure the file is a valid WebAssembly module.",
    },
    ApplicationError::DownloadFailed(url) => ApiError {
        status_code: StatusCode::BAD_GATEWAY,
        message: format!("Failed to download application from {}. Please check the URL and network connectivity.", url),
    },
    ApplicationError::AlreadyExists(app_id) => ApiError {
        status_code: StatusCode::CONFLICT,
        message: format!("Application {} already exists. Use --force to overwrite or choose a different application.", app_id),
    },
    // 其他具体错误...
}
```

#### B. 上下文创建错误 (`create_context.rs`)
**当前代码位置：** `core/crates/server/src/admin/handlers/context/create_context.rs:25-27`

**需要改进的错误处理：**
- 应用ID不存在
- 协议不支持
- 初始化参数错误
- 上下文已存在

## 4. 增强认证错误信息

### 当前问题：
- 401错误只显示`"Request failed with status: 401 Unauthorized"`
- 没有说明具体的认证失败原因
- 缺少重新认证的指导

### 改进方案：

#### A. 客户端认证错误处理
**文件：** `core/crates/meroctl/src/connection.rs:104-172`

**需要区分的认证错误类型：**
```rust
match auth_error_header {
    "token_expired" => "Your authentication token has expired. Please re-authenticate.",
    "token_revoked" => "Your authentication token has been revoked. Please re-authenticate.",
    "insufficient_permissions" => "Your account doesn't have sufficient permissions for this operation.",
    "invalid_token" => "Your authentication token is invalid. Please re-authenticate.",
    _ => "Authentication failed. Please check your credentials.",
}
```

**具体的建议信息：**
- Token过期：`"Run 'meroctl node add <node-name>' to re-authenticate"`
- 权限不足：`"Contact your administrator to request the required permissions"`
- 无效token：`"Your session may have been corrupted. Please re-add the node"`

#### B. 服务端认证中间件错误
**文件：** `core/crates/auth/src/auth/middleware.rs:85-105`

**需要返回更详细的错误信息：**
- 在响应头中包含更多错误上下文
- 区分不同类型的权限错误
- 提供API端点级别的权限说明

## 5. 错误信息本地化和用户友好性

### 改进方案：

#### A. 添加错误代码系统
```rust
pub enum MeroErrorCode {
    AUTH_001, // Token expired
    AUTH_002, // Insufficient permissions
    APP_001,  // Invalid WASM file
    APP_002,  // Application not found
    CTX_001,  // Context creation failed
    NET_001,  // Network connectivity issues
}
```

#### B. 错误信息模板
```rust
pub struct ErrorTemplate {
    pub code: MeroErrorCode,
    pub message: String,
    pub suggestion: String,
    pub documentation_link: Option<String>,
}
```

## 6. 实现优先级和具体文件修改清单

### 高优先级（立即改进）：
1. **`core/crates/meroctl/src/errors.rs`** - 增强ClientError结构
2. **`core/crates/meroctl/src/connection.rs:181-189`** - 改进HTTP错误处理
3. **`core/crates/meroctl/src/cli.rs:234-237`** - 统一错误输出格式

### 中优先级（第二阶段）：
4. **`core/crates/server/src/admin/handlers/applications/install_application.rs`** - 应用安装错误
5. **`core/crates/server/src/admin/handlers/context/create_context.rs`** - 上下文创建错误
6. **`core/crates/meroctl/src/connection.rs:104-172`** - 认证重试逻辑改进

### 低优先级（优化阶段）：
7. **`core/crates/auth/src/auth/middleware.rs`** - 服务端认证错误改进
8. **`core/crates/server/src/admin/service.rs`** - 全局错误解析改进

## 7. 测试和验证计划

### 需要测试的场景：
1. **认证错误**：Token过期、权限不足、无效token
2. **应用安装错误**：无效WASM、网络失败、应用冲突
3. **上下文创建错误**：应用不存在、参数错误
4. **输出格式**：确保JSON和Human格式的一致性
5. **错误恢复**：验证建议的解决方案确实有效

这个改进计划将显著提升错误信息的可用性，让用户能够快速理解问题并知道如何解决。
