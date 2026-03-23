use crate::{Ent, context::EntContext};

pub enum PrivacyRuleOutcome {
    Allow,
    Deny,
    Skip,
}

#[async_trait::async_trait]
pub trait EntQueryPrivacyRule<T: Ent, TCtx: EntContext>: Sync + Send {
    async fn evaluation(&self, ctx: &TCtx, ent: &T) -> PrivacyRuleOutcome;
}

#[async_trait::async_trait]
pub trait EntMutationPrivacyRule<T: Ent, TCtx: EntContext>: Sync + Send {
    async fn evaluation(&self, ctx: &TCtx, ent: &T) -> PrivacyRuleOutcome;
}

#[async_trait::async_trait]
pub trait EntPrivacyRule<TCtx: EntContext>: Send + Sync {
    async fn evaluation(&self, ctx: &TCtx) -> PrivacyRuleOutcome;
}

#[async_trait::async_trait]
impl<TEnt: Ent, T: EntPrivacyRule<TCtx>, TCtx: EntContext> EntQueryPrivacyRule<TEnt, TCtx> for T {
    async fn evaluation(&self, ctx: &TCtx, _ent: &TEnt) -> PrivacyRuleOutcome {
        self.evaluation(ctx).await
    }
}

#[async_trait::async_trait]
impl<TEnt: Ent, T: EntPrivacyRule<TCtx>, TCtx: EntContext> EntMutationPrivacyRule<TEnt, TCtx>
    for T
{
    async fn evaluation(&self, ctx: &TCtx, _ent: &TEnt) -> PrivacyRuleOutcome {
        self.evaluation(ctx).await
    }
}

pub struct AlwaysAllowRule;

#[async_trait::async_trait]
impl<TCtx: EntContext> EntPrivacyRule<TCtx> for AlwaysAllowRule {
    async fn evaluation(&self, _ctx: &TCtx) -> PrivacyRuleOutcome {
        PrivacyRuleOutcome::Allow
    }
}

pub struct AlwaysDenyRule;

#[async_trait::async_trait]
impl<TCtx: EntContext> EntPrivacyRule<TCtx> for AlwaysDenyRule {
    async fn evaluation(&self, _ctx: &TCtx) -> PrivacyRuleOutcome {
        PrivacyRuleOutcome::Deny
    }
}

pub trait EntPrivacyPolicy<TCtx: EntContext> {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<Self, TCtx>>>;
    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<Self, TCtx>>>;
}
