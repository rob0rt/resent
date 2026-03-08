use crate::{Ent, query::QueryContext};

pub enum PrivacyRuleOutcome {
    Allow,
    Deny,
    Skip,
}

#[async_trait::async_trait]
pub trait EntQueryPrivacyRule<'ctx, Ctx: 'ctx + Sync, T: Ent<'ctx, Ctx>>: Sync + Send {
    async fn evaluation(&self, ctx: &QueryContext<Ctx>, ent: &T) -> PrivacyRuleOutcome;
}

#[async_trait::async_trait]
pub trait EntMutationPrivacyRule<'ctx, Ctx: 'ctx + Sync, T: Ent<'ctx, Ctx>>: Sync + Send {
    async fn evaluation(&self, ctx: &QueryContext<Ctx>, ent: &T) -> PrivacyRuleOutcome;
}

#[async_trait::async_trait]
pub trait EntPrivacyRule<'ctx, Ctx: 'ctx + Sync>: Send + Sync {
    async fn evaluation(&self, ctx: &QueryContext<Ctx>) -> PrivacyRuleOutcome;
}

#[async_trait::async_trait]
impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>, T: EntPrivacyRule<'ctx, Ctx>>
    EntQueryPrivacyRule<'ctx, Ctx, TEnt> for T
{
    async fn evaluation(&self, ctx: &QueryContext<Ctx>, _ent: &TEnt) -> PrivacyRuleOutcome {
        self.evaluation(ctx).await
    }
}

#[async_trait::async_trait]
impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>, T: EntPrivacyRule<'ctx, Ctx>>
    EntMutationPrivacyRule<'ctx, Ctx, TEnt> for T
{
    async fn evaluation(&self, ctx: &QueryContext<Ctx>, _ent: &TEnt) -> PrivacyRuleOutcome {
        self.evaluation(ctx).await
    }
}

pub struct AlwaysAllowRule;

#[async_trait::async_trait]
impl<'ctx, Ctx: 'ctx + Sync> EntPrivacyRule<'ctx, Ctx> for AlwaysAllowRule {
    async fn evaluation(&self, _ctx: &QueryContext<Ctx>) -> PrivacyRuleOutcome {
        PrivacyRuleOutcome::Allow
    }
}

pub struct AlwaysDenyRule;

#[async_trait::async_trait]
impl<'ctx, Ctx: 'ctx + Sync> EntPrivacyRule<'ctx, Ctx> for AlwaysDenyRule {
    async fn evaluation(&self, _ctx: &QueryContext<Ctx>) -> PrivacyRuleOutcome {
        PrivacyRuleOutcome::Deny
    }
}

pub trait EntPrivacyPolicy<'ctx, Ctx: 'ctx + Sync> {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<'ctx, Ctx, Self>>>;
    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<'ctx, Ctx, Self>>>;
}
