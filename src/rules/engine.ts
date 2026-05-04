/**
 * Torot Rules Engine
 * Security detection rules written in a Lua-inspired DSL.
 * Rules are evaluated against tool output to classify findings,
 * enrich severity, and suggest fixes — without needing an LLM.
 *
 * Rule format mirrors Lua table syntax for familiarity to security researchers
 * who commonly write Lua rules for Snort/Suricata/Nmap NSE.
 */

export interface Rule {
  id:          string;
  name:        string;
  domain:      "blockchain" | "webapp" | "binary" | "api" | "all";
  severity:    "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "INFO";
  patterns:    string[];      // regex patterns to match against output lines
  negate:      string[];      // lines matching these are NOT findings
  tags:        string[];
  fix:         string;
  impact:      string;
  references:  string[];
  confidence:  number;        // 0-100
}

// ─── Built-in rule database (Lua-inspired DSL compiled to JS objects) ─────────

export const BUILTIN_RULES: Rule[] = [
  // ── Blockchain ──────────────────────────────────────────────────────────────
  {
    id: "SOL-001", name: "Reentrancy Vulnerability",
    domain: "blockchain", severity: "CRITICAL", confidence: 90,
    patterns: ["reentrancy", "re-entrancy", "ReentrancyGuard", "withdraw.*call.*balance"],
    negate:   ["ReentrancyGuard", "nonReentrant", "protected"],
    tags:     ["reentrancy", "evm", "solidity"],
    fix:      "Apply Checks-Effects-Interactions pattern. Update state BEFORE external calls. Use OpenZeppelin ReentrancyGuard.",
    impact:   "Attacker can recursively drain all ETH from the contract before balances are updated.",
    references: ["https://swcregistry.io/docs/SWC-107", "https://github.com/crytic/slither/wiki/Detector-Documentation#reentrancy-vulnerabilities"],
  },
  {
    id: "SOL-002", name: "tx.origin Authentication",
    domain: "blockchain", severity: "HIGH", confidence: 95,
    patterns: ["tx\\.origin", "tx_origin"],
    negate:   ["// tx.origin safe", "msg.sender"],
    tags:     ["auth", "phishing", "evm"],
    fix:      "Replace tx.origin with msg.sender for all authentication checks.",
    impact:   "Phishing attack lets a malicious contract bypass the owner authentication check.",
    references: ["https://swcregistry.io/docs/SWC-115"],
  },
  {
    id: "SOL-003", name: "Unchecked Integer Arithmetic",
    domain: "blockchain", severity: "HIGH", confidence: 80,
    patterns: ["overflow", "underflow", "SafeMath", "unchecked arithmetic", "integer.*overflow"],
    negate:   ["SafeMath", "solidity 0\\.8", "checked"],
    tags:     ["arithmetic", "overflow"],
    fix:      "Use Solidity 0.8+ (built-in overflow checks) or OpenZeppelin SafeMath for older versions.",
    impact:   "Integer overflow wraps the value to 0, enabling unauthorized minting or balance manipulation.",
    references: ["https://swcregistry.io/docs/SWC-101"],
  },
  {
    id: "SOL-004", name: "Unprotected selfdestruct",
    domain: "blockchain", severity: "CRITICAL", confidence: 95,
    patterns: ["selfdestruct", "suicide\\("],
    negate:   ["onlyOwner", "require.*owner", "access.*control"],
    tags:     ["selfdestruct", "evm"],
    fix:      "Restrict selfdestruct with onlyOwner/multisig guard, or remove it entirely.",
    impact:   "Any caller can permanently destroy the contract and send all ETH to themselves.",
    references: ["https://swcregistry.io/docs/SWC-106"],
  },
  {
    id: "SOL-005", name: "Timestamp Dependence",
    domain: "blockchain", severity: "MEDIUM", confidence: 75,
    patterns: ["block\\.timestamp", "now\\b"],
    negate:   ["// safe to use"],
    tags:     ["timestamp", "miner-manipulation"],
    fix:      "Use block.number instead of block.timestamp for timing logic where precision matters.",
    impact:   "Miners can manipulate block.timestamp by up to 15 seconds to influence lottery or deadline outcomes.",
    references: ["https://swcregistry.io/docs/SWC-116"],
  },
  {
    id: "SOL-006", name: "Flash Loan Attack Surface",
    domain: "blockchain", severity: "HIGH", confidence: 60,
    patterns: ["flashLoan", "flash_loan", "flashloan", "price.*oracle", "getReserves"],
    negate:   ["// flash loan protected", "reentrancy.*guard"],
    tags:     ["flash-loan", "defi", "oracle"],
    fix:      "Use TWAP oracles instead of spot prices. Add flash loan protection via ReentrancyGuard.",
    impact:   "Attacker can manipulate price oracles using flash loans to drain protocol funds in a single transaction.",
    references: ["https://immunefi.com/learn/flash-loan-attacks"],
  },
  {
    id: "SOL-007", name: "Access Control Missing",
    domain: "blockchain", severity: "HIGH", confidence: 70,
    patterns: ["access control", "missing.*modifier", "unprotected.*function", "no.*access.*control"],
    negate:   ["onlyOwner", "onlyRole", "require.*msg.sender"],
    tags:     ["access-control", "authorization"],
    fix:      "Add appropriate access control modifiers (onlyOwner, AccessControl) to privileged functions.",
    impact:   "Unauthorized callers can execute privileged operations (mint, pause, upgrade, drain).",
    references: ["https://swcregistry.io/docs/SWC-105"],
  },

  // ── Web App ──────────────────────────────────────────────────────────────────
  {
    id: "WEB-001", name: "SQL Injection",
    domain: "webapp", severity: "CRITICAL", confidence: 90,
    patterns: ["sql injection", "sqli", "SELECT.*FROM.*WHERE.*'", "1=1", "OR 1=1", "--.*sql"],
    negate:   ["parameterized", "prepared statement", "orm"],
    tags:     ["sqli", "injection", "owasp-a03"],
    fix:      "Use parameterized queries or prepared statements. Never concatenate user input into SQL.",
    impact:   "Attacker can read/modify/delete all database data, bypass authentication, or execute OS commands.",
    references: ["https://owasp.org/www-community/attacks/SQL_Injection"],
  },
  {
    id: "WEB-002", name: "XSS Vulnerability",
    domain: "webapp", severity: "HIGH", confidence: 80,
    patterns: ["xss", "cross.site.scripting", "innerHTML.*=", "document\\.write", "eval\\(.*user"],
    negate:   ["escaped", "sanitized", "DOMPurify", "htmlspecialchars"],
    tags:     ["xss", "injection", "owasp-a03"],
    fix:      "Sanitize all user input. Use Content Security Policy headers. Avoid innerHTML with untrusted data.",
    impact:   "Attacker can steal session cookies, redirect users, or perform actions on behalf of victims.",
    references: ["https://owasp.org/www-community/attacks/xss"],
  },
  {
    id: "WEB-003", name: "SSRF Vulnerability",
    domain: "webapp", severity: "HIGH", confidence: 75,
    patterns: ["ssrf", "server.side.request.forgery", "fetch.*user.*input", "curl.*param", "url.*unvalidated"],
    negate:   ["allowlist", "whitelist", "validated"],
    tags:     ["ssrf", "owasp-a10"],
    fix:      "Validate and allowlist URLs. Block requests to internal IP ranges. Use a URL parser to check host.",
    impact:   "Attacker can access internal services, cloud metadata endpoints (AWS/GCP), and exfiltrate credentials.",
    references: ["https://owasp.org/www-community/attacks/Server_Side_Request_Forgery"],
  },
  {
    id: "WEB-004", name: "Exposed Secret or Credential",
    domain: "webapp", severity: "CRITICAL", confidence: 85,
    patterns: ["api_key.*=.*['\"]\\w{20,}", "secret.*=.*['\"]\\w{10,}", "password.*=.*['\"]\\w{6,}", "AWS_SECRET", "PRIVATE_KEY"],
    negate:   ["process\\.env", "os\\.environ", "vault", "\\$\\{"],
    tags:     ["secret", "credential", "hardcoded"],
    fix:      "Move secrets to environment variables or a secrets manager (Vault, AWS Secrets Manager). Rotate immediately.",
    impact:   "Exposed credentials allow immediate unauthorized access to the affected system or cloud account.",
    references: ["https://owasp.org/www-community/vulnerabilities/Hardcoded_Password"],
  },

  // ── Binary ───────────────────────────────────────────────────────────────────
  {
    id: "BIN-001", name: "Stack Buffer Overflow",
    domain: "binary", severity: "CRITICAL", confidence: 80,
    patterns: ["buffer overflow", "stack smashing", "gets\\(", "strcpy\\(", "sprintf\\("],
    negate:   ["__stack_chk", "FORTIFY"],
    tags:     ["buffer-overflow", "memory-safety"],
    fix:      "Replace unsafe functions (gets, strcpy) with safe equivalents (fgets, strncpy). Enable stack canaries.",
    impact:   "Remote code execution via crafted input that overwrites the return address.",
    references: ["https://cwe.mitre.org/data/definitions/121.html"],
  },
  {
    id: "BIN-002", name: "Missing Security Mitigations",
    domain: "binary", severity: "MEDIUM", confidence: 90,
    patterns: ["No PIE", "No RELRO", "NX disabled", "No canary", "stack.*executable"],
    negate:   [],
    tags:     ["hardening", "exploit-mitigation"],
    fix:      "Compile with -fPIE -pie -fstack-protector-all -Wl,-z,relro,-z,now. Enable NX via -z noexecstack.",
    impact:   "Missing mitigations make exploitation significantly easier — buffer overflows become trivial RCE.",
    references: ["https://blog.trailofbits.com/2019/11/29/understanding-checksec"],
  },

  // ── API ──────────────────────────────────────────────────────────────────────
  {
    id: "API-001", name: "Broken Object Level Authorization (IDOR)",
    domain: "api", severity: "HIGH", confidence: 65,
    patterns: ["idor", "bola", "object.*level.*auth", "user_id.*param", "account.*id.*unvalidated"],
    negate:   ["authorized", "ownership.*check", "permission.*check"],
    tags:     ["idor", "bola", "owasp-api1"],
    fix:      "Validate that the authenticated user owns the requested resource on every API call.",
    impact:   "Attacker can read, modify, or delete any other user's data by changing an ID parameter.",
    references: ["https://owasp.org/www-project-api-security"],
  },
  {
    id: "API-002", name: "JWT Vulnerability",
    domain: "api", severity: "HIGH", confidence: 80,
    patterns: ["jwt", "alg.*none", "algorithm.*none", "HS256.*RS256", "weak.*jwt.*secret"],
    negate:   ["RS256", "ES256", "strong.*secret"],
    tags:     ["jwt", "authentication", "owasp-api2"],
    fix:      "Use RS256/ES256. Validate algorithm explicitly. Use a strong secret (32+ random bytes).",
    impact:   "Attacker can forge JWT tokens and authenticate as any user including administrators.",
    references: ["https://portswigger.net/web-security/jwt"],
  },
];

// ─── Rule engine ──────────────────────────────────────────────────────────────

export interface RuleMatch {
  rule:    Rule;
  matched: string[];   // lines that triggered the rule
  score:   number;
}

export function applyRules(
  output:  string,
  domain?: "blockchain" | "webapp" | "binary" | "api" | "all",
): RuleMatch[] {
  const lines   = output.split("\n");
  const matches: RuleMatch[] = [];

  const rules = domain
    ? BUILTIN_RULES.filter((r) => r.domain === domain || r.domain === "all")
    : BUILTIN_RULES;

  for (const rule of rules) {
    const matchedLines: string[] = [];
    const patternREs = rule.patterns.map((p) => new RegExp(p, "i"));
    const negateREs  = rule.negate.map((p) => new RegExp(p, "i"));

    for (const line of lines) {
      const triggered = patternREs.some((re) => re.test(line));
      const negated   = negateREs.some((re) => re.test(line));
      if (triggered && !negated) {
        matchedLines.push(line.trim());
      }
    }

    if (matchedLines.length > 0) {
      matches.push({
        rule,
        matched: matchedLines.slice(0, 5),
        score:   rule.confidence,
      });
    }
  }

  return matches.sort((a, b) => {
    const order = (s: string) =>
      ({ CRITICAL: 0, HIGH: 1, MEDIUM: 2, LOW: 3, INFO: 4 })[s] ?? 5;
    return order(a.rule.severity) - order(b.rule.severity);
  });
}

export function loadCustomRules(json: unknown): Rule[] {
  if (!Array.isArray(json)) return [];
  return json.filter((r) => r.id && r.name && r.patterns);
}
