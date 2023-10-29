use std::borrow::Cow;

use backend::{uri_to_pathbuf, Backend};
use serde_yaml::to_string;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService, Server};
use tracing::{info, warn};

mod backend;
mod parser;

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        init_params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        info!(
            "initializing server\n {}",
            serde_yaml::to_string(&init_params).unwrap()
        );
        self.client
            .log_message(MessageType::INFO, "initializing server")
            .await;

        let workspace_folder = init_params.workspace_folders.unwrap_or_default();
        for workspace in workspace_folder.iter() {
            self.process_workspace(workspace).unwrap_or_else(|x| {
                warn!("{x}");
            });
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        // change: Some(TextDocumentSyncKind::FULL),
                        change: Some(TextDocumentSyncKind::NONE),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                inlay_hint_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, init_params: InitializedParams) {
        info!("call initialized\n {}", to_string(&init_params).unwrap());
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }
    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        info!("call completion");
        let _ = params;
        info!("\n{}", serde_yaml::to_string(&params).unwrap());
        let ret = CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: self.completions(),
        });
        return Ok(Some(ret));
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let path = uri_to_pathbuf(&params.text_document.uri);

        match path {
            Ok(v) => self.index_file(v, params.text_document.text),
            Err(e) => warn!("Can't index file {} {:#?}", params.text_document.uri, e),
        }
    }
    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let path = uri_to_pathbuf(&params.text_document.uri);

        let path = match path {
            Ok(v) => v,
            Err(e) => {
                warn!("Can't inlay_hint {:#?}: {e:#?}", params.text_document.uri);
                return Err(Error {
                    code: ErrorCode::InvalidParams,
                    message: Cow::from("invalid uri"),
                    data: None,
                });
            }
        };
        info!("Got a textDocument/inlayHint request on {path:#?}");

        let step_lines = self.step_lines.get(&path);
        let step_lines = match step_lines {
            Some(v) => v.to_owned(),
            None => {
                warn!("No content found for file");
                return Err(Error {
                    code: ErrorCode::InvalidParams,
                    message: Cow::from("no content"),
                    data: None,
                });
            }
        };

        if step_lines.is_empty() {
            return Ok(None);
        }

        Ok(Some(
            step_lines
                .into_iter()
                .enumerate()
                .map(|(i, val)| InlayHint {
                    position: Position {
                        line: val as u32,
                        character: 0,
                    },
                    label: From::from(format!("step {}: ", i + 1)),
                    kind: None,
                    text_edits: None,
                    tooltip: None,
                    padding_left: None,
                    padding_right: None,
                    data: None,
                })
                .collect(),
        ))
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let pathbuf = match uri_to_pathbuf(&params.text_document.uri) {
            Ok(v) => v,
            Err(e) => {
                warn!("{e:#?}");
                return;
            }
        };

        let document_data = match params.content_changes.get(0) {
            Some(v) => &v.text,
            None => {
                warn!("no text send to did_change");
                return;
            }
        };
        self.index_file(pathbuf, document_data.to_owned())

        // Calling self.process_str here does weird behavior with autocompletion: it will
        // made the IDE try to autocomplete the current word with itself
        //     match self.process_str(&document_data, &pathbuf) {
        //         Ok(_) => {}
        //         Err(e) => {
        //             warn!("can't process text send to did_change: {e:#?}")
        //         }
        //     }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let pathbuf = match uri_to_pathbuf(&params.text_document.uri) {
            Ok(v) => v,
            Err(e) => {
                warn!("{e:#?}");
                return;
            }
        };

        let document_data = match params.text {
            Some(v) => v,
            None => {
                warn!("no text send in did_save");
                return;
            }
        };
        match self.process_str(&document_data, &pathbuf) {
            Ok(_) => {}
            Err(e) => {
                warn!("can't process text send to did_save: {e:#?}")
            }
        };
    }
}

#[tokio::main]
async fn main() {
    // setup tracing
    let file_appender = tracing_appender::rolling::daily("/tmp", "cooklang.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
