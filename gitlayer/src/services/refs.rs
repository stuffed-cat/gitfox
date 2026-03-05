//! Reference service implementation

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::config::Config;
use crate::git::repository::RepositoryOps;
use crate::git::refs::RefOps;
use crate::proto::*;

pub struct RefServiceImpl {
    config: Arc<Config>,
}

impl RefServiceImpl {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
    
    fn get_repo_path(&self, repo: Option<&Repository>) -> Result<String, Status> {
        let repo = repo.ok_or_else(|| Status::invalid_argument("Repository required"))?;
        
        if !repo.storage_path.is_empty() {
            Ok(repo.storage_path.clone())
        } else if !repo.relative_path.is_empty() {
            Ok(self.config.repo_path(&repo.relative_path))
        } else {
            Err(Status::invalid_argument("Repository path required"))
        }
    }
}

#[tonic::async_trait]
impl ref_service_server::RefService for RefServiceImpl {
    async fn list_branches(
        &self,
        request: Request<ListBranchesRequest>,
    ) -> Result<Response<ListBranchesResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let pattern = if req.pattern.is_empty() { None } else { Some(req.pattern.as_str()) };
        let limit = if req.limit > 0 { Some(req.limit as usize) } else { None };
        let offset = if req.offset > 0 { Some(req.offset as usize) } else { None };
        
        // 先计算总数（不带分页）
        let all_branches = RefOps::list_branches(&repo, pattern, None, None)?;
        let total_count = all_branches.len() as u64;
        
        // 再获取分页后的结果
        let branches = RefOps::list_branches(&repo, pattern, limit, offset)?;
        
        let branch_protos: Vec<Branch> = branches.into_iter()
            .map(|b| Branch {
                name: b.name,
                commit_id: b.commit_id,
                is_default: b.is_head,
                is_protected: false, // 分支保护检查由 Main App gRPC Auth 服务处理
            })
            .collect();
        
        Ok(Response::new(ListBranchesResponse {
            branches: branch_protos,
            total_count: total_count as i32,
        }))
    }
    
    async fn create_branch(
        &self,
        request: Request<CreateBranchRequest>,
    ) -> Result<Response<CreateBranchResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match RefOps::create_branch(&repo, &req.name, &req.start_point) {
            Ok(branch) => Ok(Response::new(CreateBranchResponse {
                success: true,
                branch: Some(Branch {
                    name: branch.name,
                    commit_id: branch.commit_id,
                    is_default: false,
                    is_protected: false,
                }),
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(CreateBranchResponse {
                success: false,
                branch: None,
                message: e.to_string(),
            })),
        }
    }
    
    async fn delete_branch(
        &self,
        request: Request<DeleteBranchRequest>,
    ) -> Result<Response<DeleteBranchResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match RefOps::delete_branch(&repo, &req.name, req.force) {
            Ok(_) => Ok(Response::new(DeleteBranchResponse {
                success: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(DeleteBranchResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }
    
    async fn list_tags(
        &self,
        request: Request<ListTagsRequest>,
    ) -> Result<Response<ListTagsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let pattern = if req.pattern.is_empty() { None } else { Some(req.pattern.as_str()) };
        let limit = if req.limit > 0 { Some(req.limit as usize) } else { None };
        let offset = if req.offset > 0 { Some(req.offset as usize) } else { None };
        
        let tags = RefOps::list_tags(&repo, pattern, limit, offset)?;
        
        let tag_protos: Vec<Tag> = tags.into_iter()
            .map(|t| Tag {
                name: t.name,
                target_id: t.target_id,
                message: t.message.unwrap_or_default(),
                tagger_name: t.tagger_name.unwrap_or_default(),
                tagger_email: t.tagger_email.unwrap_or_default(),
                tagger_date: t.tagger_time.unwrap_or(0),
                is_annotated: t.is_annotated,
            })
            .collect();
        
        Ok(Response::new(ListTagsResponse {
            tags: tag_protos,
            total_count: 0,
        }))
    }
    
    async fn create_tag(
        &self,
        request: Request<CreateTagRequest>,
    ) -> Result<Response<CreateTagResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let message = if req.message.is_empty() { None } else { Some(req.message.as_str()) };
        let tagger_name = if req.tagger_name.is_empty() { None } else { Some(req.tagger_name.as_str()) };
        let tagger_email = if req.tagger_email.is_empty() { None } else { Some(req.tagger_email.as_str()) };
        
        match RefOps::create_tag(&repo, &req.name, &req.target, message, tagger_name, tagger_email) {
            Ok(tag) => Ok(Response::new(CreateTagResponse {
                success: true,
                tag: Some(Tag {
                    name: tag.name,
                    target_id: tag.target_id,
                    message: tag.message.unwrap_or_default(),
                    tagger_name: tag.tagger_name.unwrap_or_default(),
                    tagger_email: tag.tagger_email.unwrap_or_default(),
                    tagger_date: tag.tagger_time.unwrap_or(0),
                    is_annotated: tag.is_annotated,
                }),
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(CreateTagResponse {
                success: false,
                tag: None,
                message: e.to_string(),
            })),
        }
    }
    
    async fn delete_tag(
        &self,
        request: Request<DeleteTagRequest>,
    ) -> Result<Response<DeleteTagResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match RefOps::delete_tag(&repo, &req.name) {
            Ok(_) => Ok(Response::new(DeleteTagResponse {
                success: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(DeleteTagResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }
    
    async fn find_ref(
        &self,
        request: Request<FindRefRequest>,
    ) -> Result<Response<FindRefResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match RefOps::find_ref(&repo, &req.ref_name) {
            Ok(Some(r)) => Ok(Response::new(FindRefResponse {
                r#ref: Some(Ref {
                    name: r.name,
                    target: r.target,
                    symbolic_target: r.symbolic_target.unwrap_or_default(),
                    is_symbolic: r.is_symbolic,
                }),
                found: true,
            })),
            Ok(None) => Ok(Response::new(FindRefResponse {
                r#ref: None,
                found: false,
            })),
            Err(_) => Ok(Response::new(FindRefResponse {
                r#ref: None,
                found: false,
            })),
        }
    }
    
    async fn update_ref(
        &self,
        request: Request<UpdateRefRequest>,
    ) -> Result<Response<UpdateRefResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let old_value = if req.old_value.is_empty() { None } else { Some(req.old_value.as_str()) };
        
        match RefOps::update_ref(&repo, &req.ref_name, old_value, &req.new_value) {
            Ok(_) => Ok(Response::new(UpdateRefResponse {
                success: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(UpdateRefResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }
    
    async fn list_refs(
        &self,
        request: Request<ListRefsRequest>,
    ) -> Result<Response<ListRefsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let pattern = if req.pattern.is_empty() { None } else { Some(req.pattern.as_str()) };
        
        let refs = RefOps::list_refs(&repo, pattern)?;
        
        let ref_protos: Vec<Ref> = refs.into_iter()
            .map(|r| Ref {
                name: r.name,
                target: r.target,
                symbolic_target: r.symbolic_target.unwrap_or_default(),
                is_symbolic: r.is_symbolic,
            })
            .collect();
        
        Ok(Response::new(ListRefsResponse { refs: ref_protos }))
    }
}
