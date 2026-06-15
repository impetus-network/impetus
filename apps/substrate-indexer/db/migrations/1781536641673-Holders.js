module.exports = class Holders1781536641673 {
    name = 'Holders1781536641673'

    async up(db) {
        await db.query(`CREATE TABLE "holder" ("id" character varying NOT NULL, "free" numeric NOT NULL, "reserved" numeric NOT NULL, "total" numeric NOT NULL, "nonce" integer NOT NULL, "updated_at" integer NOT NULL, CONSTRAINT "PK_holder_id" PRIMARY KEY ("id"))`)
        await db.query(`CREATE INDEX "IDX_holder_total" ON "holder" ("total") `)
        await db.query(`CREATE TABLE "chain_stat" ("id" character varying NOT NULL, "total_issuance" numeric NOT NULL, "holders_count" integer NOT NULL, "seeded" boolean NOT NULL, "updated_at" integer NOT NULL, CONSTRAINT "PK_chain_stat_id" PRIMARY KEY ("id"))`)
    }

    async down(db) {
        await db.query(`DROP INDEX "public"."IDX_holder_total"`)
        await db.query(`DROP TABLE "holder"`)
        await db.query(`DROP TABLE "chain_stat"`)
    }
}
