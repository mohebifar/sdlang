// drizzle-intentlang, spec/index.spec: the database provider used by SPEC.md's worked
// example (`use "drizzle" for database { dialect: "postgres" }`). Not part of the first
// milestone; it shows the binding vocabulary in full.

export type DrizzleOptions = {
  dialect: "postgres" | "sqlite"
}

// What the `database` binding owns once a project writes `use "drizzle" for database`.
export binding database {
  options: DrizzleOptions
  outputs {
    config: "drizzle.config.ts"
    schema_index: "src/db/schema/index.ts"
  }
  collects: select entity
  retains {
    migrations {
      path: "drizzle/"
      producer: "generator"
    }
    backfills {
      path: "drizzle/backfills/"
      producer: "agent"
    }
  }
  generator: migrate
  observer: ledger
  checks: [schema_matches]
}

// How entities are materialised: one schema module each, written by the generator.
export implements entity {
  outputs {
    schema: "src/db/schema/${meta.name}.ts"
  }
  exports {
    table {
      output: "schema"
      symbol: "default"
      mode: "value"
    }
    row {
      output: "schema"
      symbol: "Row"
      mode: "type"
    }
  }
  generator: migrate
  checks: [schema_matches]
}

export annotation backfill {
  targets: ["entity_field"]
  projection: "public"
  repeatable: false
  parameters {
    id: string
  }
}

export annotation index {
  targets: ["entity_field"]
  projection: "public"
  repeatable: false
  parameters {
    method: "btree" | "hash" = "btree"
  }
}

export check schema_matches {
  purpose: "conformance"
  scope: "declaration"
  run: ["node", "./checks/schema-matches.js"]
}

export generator migrate {
  run: ["node", "./gen/migrate.js"]
}

export observer ledger {
  run: ["node", "./observe/ledger.js"]
}
