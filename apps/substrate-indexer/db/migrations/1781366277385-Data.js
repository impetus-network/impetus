module.exports = class Data1781366277385 {
    name = 'Data1781366277385'

    async up(db) {
        await db.query(`CREATE TABLE "validator" ("id" character varying NOT NULL, "commission" integer NOT NULL, "blocked" boolean NOT NULL, "active" boolean NOT NULL, "elected" boolean NOT NULL, "self_bonded" numeric NOT NULL, "blocks_produced" integer NOT NULL, "last_block" integer, "updated_at" integer NOT NULL, CONSTRAINT "PK_ae0a943022c24bd60e7161e0fad" PRIMARY KEY ("id"))`)
        await db.query(`CREATE TABLE "nominator" ("id" character varying NOT NULL, "targets" text array NOT NULL, "active" boolean NOT NULL, "updated_at" integer NOT NULL, CONSTRAINT "PK_7489b7a79b066f2660eab25f60b" PRIMARY KEY ("id"))`)
        await db.query(`CREATE TABLE "era" ("id" character varying NOT NULL, "index" integer NOT NULL, "validator_reward" numeric, "start_block" integer NOT NULL, CONSTRAINT "PK_a30749cdf0189d890a8dbc9aa7d" PRIMARY KEY ("id"))`)
        await db.query(`CREATE TABLE "payout" ("id" character varying NOT NULL, "validator" text NOT NULL, "era" integer NOT NULL, "block" integer NOT NULL, CONSTRAINT "PK_1cb73ce021dc6618a3818b0a474" PRIMARY KEY ("id"))`)
        await db.query(`CREATE INDEX "IDX_556a3ed64f0a371f638abe95c6" ON "payout" ("validator") `)
        await db.query(`CREATE TABLE "pool" ("id" character varying NOT NULL, "creator" text NOT NULL, "state" text NOT NULL, "created_block" integer NOT NULL, "updated_at" integer NOT NULL, CONSTRAINT "PK_db1bfe411e1516c01120b85f8fe" PRIMARY KEY ("id"))`)
        await db.query(`CREATE TABLE "pool_member" ("id" character varying NOT NULL, "pool_id" integer NOT NULL, "bonded" numeric NOT NULL, "last_claim_block" integer, "updated_at" integer NOT NULL, CONSTRAINT "PK_3cb44e7780c511cd22021d2d6ad" PRIMARY KEY ("id"))`)
        await db.query(`CREATE INDEX "IDX_fb6d63cd5f27710b5b8e1ab76e" ON "pool_member" ("pool_id") `)
        await db.query(`CREATE TABLE "stake_event" ("id" character varying NOT NULL, "account" text NOT NULL, "kind" text NOT NULL, "amount" numeric NOT NULL, "block" integer NOT NULL, "timestamp" TIMESTAMP WITH TIME ZONE NOT NULL, CONSTRAINT "PK_991cc0713b262d0c22242590628" PRIMARY KEY ("id"))`)
        await db.query(`CREATE INDEX "IDX_91cf475844a40f783bf8f84962" ON "stake_event" ("account") `)
        await db.query(`CREATE TABLE "gasless_rule" ("id" character varying NOT NULL, "contract" text NOT NULL, "selector" text NOT NULL, "enabled" boolean NOT NULL, "min_value" numeric NOT NULL, "updated_at_block" integer NOT NULL, CONSTRAINT "PK_ca16f2675038bcb8754fb321d47" PRIMARY KEY ("id"))`)
        await db.query(`CREATE INDEX "IDX_e00acdc817da72383a937ded0c" ON "gasless_rule" ("contract") `)
        await db.query(`CREATE TABLE "transfer" ("id" character varying NOT NULL, "from" text NOT NULL, "to" text NOT NULL, "amount" numeric NOT NULL, "block" integer NOT NULL, "timestamp" TIMESTAMP WITH TIME ZONE NOT NULL, "extrinsic_hash" text, CONSTRAINT "PK_fd9ddbdd49a17afcbe014401295" PRIMARY KEY ("id"))`)
        await db.query(`CREATE INDEX "IDX_be54ea276e0f665ffc38630fc0" ON "transfer" ("from") `)
        await db.query(`CREATE INDEX "IDX_4cbc37e8c3b47ded161f44c24f" ON "transfer" ("to") `)
    }

    async down(db) {
        await db.query(`DROP INDEX "public"."IDX_4cbc37e8c3b47ded161f44c24f"`)
        await db.query(`DROP INDEX "public"."IDX_be54ea276e0f665ffc38630fc0"`)
        await db.query(`DROP TABLE "transfer"`)
        await db.query(`DROP INDEX "public"."IDX_e00acdc817da72383a937ded0c"`)
        await db.query(`DROP TABLE "gasless_rule"`)
        await db.query(`DROP INDEX "public"."IDX_91cf475844a40f783bf8f84962"`)
        await db.query(`DROP TABLE "stake_event"`)
        await db.query(`DROP INDEX "public"."IDX_fb6d63cd5f27710b5b8e1ab76e"`)
        await db.query(`DROP TABLE "pool_member"`)
        await db.query(`DROP TABLE "pool"`)
        await db.query(`DROP INDEX "public"."IDX_556a3ed64f0a371f638abe95c6"`)
        await db.query(`DROP TABLE "payout"`)
        await db.query(`DROP TABLE "era"`)
        await db.query(`DROP TABLE "nominator"`)
        await db.query(`DROP TABLE "validator"`)
    }
}
