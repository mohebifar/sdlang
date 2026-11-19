// @intentlang/nextjs, spec/index.spec: the first milestone's only adapter.
// Supported: one App Router layout, static routes only (route_path admits nothing else in
// v0), and the exact toolchain pinned by the package's own lock. Path templates use the
// core substitution rule for route_path (SPEC.md, Packages -> Kind definitions).

export kind page {
  fields {
    route: route_path @interface
    calls: [ref action] @relation("calls")
    reads: [ref entity] @relation("reads")
    requires: [ref catalog] @relation("requires")
  }
  outputs {
    page: "src/app/${route}/page.tsx"
    tests: "tests/app/${route}/page.test.ts"
  }
  creates {
    parts: "src/app/${route}/_parts/"
  }
  exports {
    page {
      output: "page"
      symbol: "default"
      mode: "value"
    }
  }
  ambient {
    imports: ["next/link", "next/navigation", "react", "react/jsx-runtime"]
  }
  inputs: ["src/app/layout.tsx", "src/middleware.ts", "next.config.ts"]
  checks: [typecheck, build, test, conformance]
  skills: [app_router]
}

// How this adapter materialises the core action kind: one server module per action.
export implements action {
  outputs {
    server: "src/server/${meta.module}/${meta.name}.ts"
    tests: "tests/server/${meta.module}/${meta.name}.test.ts"
  }
  exports {
    action {
      output: "server"
      symbol: "default"
      mode: "value"
    }
    contract {
      output: "server"
      symbol: "Input"
      mode: "type"
    }
  }
  ambient {
    imports: ["server-only"]
  }
  checks: [typecheck, test, conformance]
}

// Toolchain files owned by the stack declaration.
export implements stack {
  outputs {
    manifest: "package.json"
    lockfile: "pnpm-lock.yaml"
    tsconfig: "tsconfig.json"
    next_config: "next.config.ts"
  }
  generator: install
}

export check typecheck {
  purpose: "build"
  scope: "project"
  run: ["pnpm", "exec", "tsc", "--noEmit"]
}

export check build {
  purpose: "build"
  scope: "project"
  run: ["pnpm", "exec", "next", "build"]
}

export check test {
  purpose: "test"
  scope: "declaration"
  run: ["pnpm", "exec", "vitest", "run", "--reporter", "./checks/vitest-cases.js"]
}

export check conformance {
  purpose: "conformance"
  scope: "declaration"
  run: ["node", "./checks/conformance.js"]
}

export generator install {
  run: ["node", "./gen/install.js"]
}
