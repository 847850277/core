use std::io;
use std::sync::Arc;

use calimero_primitives::application::{
    Application, ApplicationBlob, ApplicationId, ApplicationSource,
};
use calimero_primitives::blobs::BlobId;
use calimero_primitives::hash::Hash;
use calimero_store::{key, types};
use camino::Utf8PathBuf;
use eyre::bail;
use futures_util::TryStreamExt;
use reqwest::Url;
use tokio::fs::File;
use tokio_util::compat::TokioAsyncReadCompatExt;

use super::NodeClient;

impl NodeClient {
    pub fn get_application(
        &self,
        application_id: &ApplicationId,
    ) -> eyre::Result<Option<Application>> {
        let handle = self.datastore.handle();

        let key = key::ApplicationMeta::new(*application_id);

        let Some(application) = handle.get(&key)? else {
            return Ok(None);
        };

        let application = Application::new(
            *application_id,
            ApplicationBlob {
                bytecode: application.bytecode.blob_id(),
                compiled: application.compiled.blob_id(),
            },
            application.size,
            application.source.parse()?,
            application.metadata.into_vec(),
        );

        Ok(Some(application))
    }

    pub async fn get_application_bytes(
        &self,
        application_id: &ApplicationId,
    ) -> eyre::Result<Option<Arc<[u8]>>> {
        let handle = self.datastore.handle();

        let key = key::ApplicationMeta::new(*application_id);

        let Some(application) = handle.get(&key)? else {
            return Ok(None);
        };

        let Some(bytes) = self
            .get_blob_bytes(&application.bytecode.blob_id(), None)
            .await?
        else {
            bail!("fatal: application points to dangling blob");
        };

        Ok(Some(bytes))
    }

    pub fn has_application(&self, application_id: &ApplicationId) -> eyre::Result<bool> {
        let handle = self.datastore.handle();

        let key = key::ApplicationMeta::new(*application_id);

        if let Some(application) = handle.get(&key)? {
            return self.has_blob(&application.bytecode.blob_id());
        }

        Ok(false)
    }

    pub fn install_application(
        &self,
        blob_id: &BlobId,
        size: u64,
        source: &ApplicationSource,
        metadata: Vec<u8>,
    ) -> eyre::Result<ApplicationId> {
        let application = types::ApplicationMeta::new(
            key::BlobMeta::new(*blob_id),
            size,
            source.to_string().into_boxed_str(),
            metadata.into_boxed_slice(),
            key::BlobMeta::new(BlobId::from([0; 32])),
        );

        let application_id = {
            // 方案一：从 ApplicationId 计算中排除 source 字段
            // 确保 ApplicationId 仅基于应用内容，而不依赖于安装路径
            let components = (
                application.bytecode,     // BlobId - 基于文件内容的哈希
                application.size,         // 文件大小
                &application.metadata,    // 应用元数据
                // 注意：故意排除 &application.source 以避免路径依赖
            );

            ApplicationId::from(*Hash::hash_borsh(&components)?)
        };

        let mut handle = self.datastore.handle();

        let key = key::ApplicationMeta::new(application_id);

        handle.put(&key, &application)?;

        Ok(application_id)
    }

    pub async fn install_application_from_path(
        &self,
        path: Utf8PathBuf,
        metadata: Vec<u8>,
    ) -> eyre::Result<ApplicationId> {
        let path = path.canonicalize_utf8()?;

        let file = File::open(&path).await?;

        let expected_size = file.metadata().await?.len();

        let (blob_id, size) = self
            .add_blob(file.compat(), Some(expected_size), None)
            .await?;

        let Ok(uri) = Url::from_file_path(path) else {
            bail!("non-absolute path")
        };
        // 打印输入metadata
        println!("Installing application from path: {}", uri);
        println!("Metadata: {:?}", metadata);

        self.install_application(&blob_id, size, &uri.as_str().parse()?, metadata)

        // // 方案一：使用通用本地标识符，隐藏实际文件路径
        // tracing::info!("🔍 方案一工作原理详解:");
        // tracing::info!("  📁 原始文件路径: {}", path.as_str());
        // tracing::info!("  📦 文件已上传为 Blob: {}", blob_id);
        // tracing::info!("  📏 文件大小: {} bytes", size);
        //
        // // 关键点1: ApplicationSource 本质上就是一个 Url 的包装器
        // let local_source: ApplicationSource = "local://application".parse()?;
        // tracing::info!("  🔗 ApplicationSource 本质: Url 包装器");
        // tracing::info!("  🎭 隐私保护策略: '{}' -> '{}'", path.as_str(), local_source);
        //
        // // 关键点：验证隐私保护的有效性
        // tracing::info!("  �️ 隐私保护验证:");
        // tracing::info!("     • 原始路径: {}", path.as_str());
        // tracing::info!("     • 存储的source: {}", local_source);
        // tracing::info!("     • 路径是否相等: {}", path.as_str() == local_source.to_string());
        // tracing::info!("     • 能否从source反推路径: ❌ 不可能");
        // tracing::info!("     • 用户隐私是否受保护: ✅ 完全保护");
        //
        // // 关键点2: source 字段只用于显示和审计，不影响应用程序的查找和执行
        // tracing::info!("  🎯 关键原理: source 字段仅用于显示，应用查找依赖 ApplicationId");
        // tracing::info!("  🆔 ApplicationId 计算: 基于文件内容(blob_id) + 大小 + 元数据，不包含 source");
        //
        // // 关键点3: 实际的应用程序文件内容已经存储在 blob 系统中
        // tracing::info!("  💾 文件存储: 应用程序字节码已安全存储在 Blob 系统中");
        // tracing::info!("  🔍 文件查找: 通过 blob_id({}) 可以找到实际的 WASM 字节码", blob_id);
        //
        // // 关键点4: 解释为什么这种替换是安全的
        // tracing::info!("  ✅ 安全性保证:");
        // tracing::info!("     • 应用执行时使用 blob_id 获取字节码，与 source 无关");
        // tracing::info!("     • ApplicationId 不依赖 source，确保应用身份的一致性");
        // tracing::info!("     • 本地路径信息完全隐藏，保护用户隐私");
        // tracing::info!("     • 应用功能完全不受影响，只是来源显示被统一化");
        //
        // self.install_application(&blob_id, size, &local_source, metadata)
    }

    pub async fn install_application_from_url(
        &self,
        url: Url,
        metadata: Vec<u8>,
        expected_hash: Option<&Hash>,
    ) -> eyre::Result<ApplicationId> {
        let uri = url.as_str().parse()?;

        let response = reqwest::Client::new().get(url).send().await?;

        let expected_size = response.content_length();

        let (blob_id, size) = self
            .add_blob(
                response
                    .bytes_stream()
                    .map_err(io::Error::other)
                    .into_async_read(),
                expected_size,
                expected_hash,
            )
            .await?;

        // 打印输入metadata
        println!("Installing application from path: {}", uri);
        println!("Metadata: {:?}", metadata);

        self.install_application(&blob_id, size, &uri, metadata)
    }

    pub fn uninstall_application(&self, application_id: &ApplicationId) -> eyre::Result<()> {
        let mut handle = self.datastore.handle();

        let key = key::ApplicationMeta::new(*application_id);

        handle.delete(&key)?;

        Ok(())
    }

    pub fn list_applications(&self) -> eyre::Result<Vec<Application>> {
        let handle = self.datastore.handle();

        let mut iter = handle.iter::<key::ApplicationMeta>()?;

        let mut applications = vec![];

        for (id, app) in iter.entries() {
            let (id, app) = (id?, app?);
            applications.push(Application::new(
                id.application_id(),
                ApplicationBlob {
                    bytecode: app.bytecode.blob_id(),
                    compiled: app.compiled.blob_id(),
                },
                app.size,
                app.source.parse()?,
                app.metadata.to_vec(),
            ));
        }

        Ok(applications)
    }

    pub fn update_compiled_app(
        &self,
        application_id: &ApplicationId,
        compiled_blob_id: &BlobId,
    ) -> eyre::Result<()> {
        let mut handle = self.datastore.handle();

        let key = key::ApplicationMeta::new(*application_id);

        let Some(mut application) = handle.get(&key)? else {
            bail!("application not found");
        };

        application.compiled = key::BlobMeta::new(*compiled_blob_id);

        handle.put(&key, &application)?;

        Ok(())
    }
}
