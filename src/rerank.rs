use anyhow::Result;
use fastembed::{RerankInitOptions, RerankerModel, TextRerank};

pub const DEFAULT_RERANKER_MODEL: &str = "jinaai/jina-reranker-v1-turbo-en";

fn model_from_name(name: &str) -> RerankerModel {
    match name {
        "BAAI/bge-reranker-base" => RerankerModel::BGERerankerBase,
        "BAAI/bge-reranker-v2-m3" => RerankerModel::BGERerankerV2M3,
        "jinaai/jina-reranker-v1-turbo-en" => RerankerModel::JINARerankerV1TurboEn,
        "jinaai/jina-reranker-v2-base-multilingual" => RerankerModel::JINARerankerV2BaseMultiligual,
        _ => RerankerModel::JINARerankerV1TurboEn,
    }
}

/// Create a TextRerank instance. Returns None on init failure (caller should fall back to fusion score).
pub fn try_create_reranker(model: &str) -> Result<Option<TextRerank>> {
    let reranker_model = model_from_name(model);
    let mut opts = RerankInitOptions::new(reranker_model).with_show_download_progress(true);
    if let Ok(cache_dir) = std::env::var("MEMKIT_MODEL_CACHE") {
        opts = opts.with_cache_dir(cache_dir.into());
    }
    match TextRerank::try_new(opts) {
        Ok(r) => Ok(Some(r)),
        Err(e) => {
            crate::term::warn(format!(
                "warning: reranker init failed ({}), falling back to fusion score",
                e
            ));
            Ok(None)
        }
    }
}
