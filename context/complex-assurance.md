Key Takeaways

1. **AI deployments require assured guarantees**, not informal convention & misplaced trust
2. **Policy definition is mandatory** for critical environments
3. **Guardrails can guide completion**, not just prevent disasters
4. **Context-aware policies** and guardrails enable nuanced permission management
5. **AI guardrailing AI** is a viable overwatch capability for ‘the unknown’

—

There are two ways to interpret and answer the questions:

1. **How do we prevent the actual scenarios?**
   - Replit DB delete
   - Gemini CLI file wipe
2. **How do we address the more nuanced questions being asked:**
   - **Permission Context**: How can we distinguish between legitimate and problematic uses of a permissioned capability?
   - **Dynamic Procedures**: Can guardrails guide agents through dynamic procedures (not defined in existing policy or sop)?

### **Part 1\. How we prevent the actual scenarios (with video)**

First, why it matters and will sell: AI agents face fundamental limitations.

- **Modern AI: The Alignment Gap and Resource Limitations:** Transformer and reinforcement learning models face a critical alignment gap, where optimization for narrow tasks and recent context diverges from true global intentions, especially in large state-spaces. Concurrently, finite GPU and attention resources limit their operational context, preventing a holistic view of a system. It goes without saying, interpretability is still a problem and until it isn’t \- we cannot misplace trust in these tools…
- **Classical Human: Misplaced trust / "Vibes-based" deployment is dangerous**: Misunderstanding or confusing AI’s intelligence as an optimization for your intent vs its internal reward signal, leads to things such as informal and unsafe conventions like ("CODE FREEZE") comments, or assumptions like (“Obviously the smartest _thing_ on the planet wouldn’t delete the critical DB”).

**1\. Replit Database Deletion**

- **What happened**: Agent dropped production database despite developer's "CODE FREEZE" convention
- **Root cause**: Informal rules the AI ignored \+ unprotected production database

**2\. Gemini CLI File Destruction**

- **What happened**: Catastrophic file system operations without version control
- **Root cause**: No version control \+ unprotected file system

**Our solution**: EQTY transforms natural language rules into deterministic technical enforcement at the tool execution level in the agent runtime, and in a deployed policy-enforcement gateway. When developers write rules like "CODE FREEZE" or "require version control before file operations," EQTY intercepts every AI action and enforces these policies before execution occurs. This converts hope-based deployment ("the AI should follow our rules") into guarantee-based deployment ("the AI cannot violate our rules").

**Note**: The question posed "agent with legitimate permissions to use delete" but in reality, these agents violated intended rules. The nuance of distinguishing legitimate vs illegitimate use of the same permission is addressed in Part 2 (also shown in video).

### **Live Demonstration**

**Watch EQTY prevent the Replit scenario:**

- [Video 1 \- Main Demo (starts at 2:32)](https://www.loom.com/share/41490af57be344c297dbef415bbcf6eb?t=152&sid=896bbfed-7681-4df6-9e57-f496df233f30) \- video cuts out due to limits, proceed to video 2 after
- [Video 2 \- Continuation](https://www.loom.com/share/23d0a989bf8549f88f92436b69731315?sid=719336b8-47ea-4dca-a023-dfd7b239c433)

**Key demonstrations:**

- "CODE FREEZE" text becomes enforceable rule
- Smart contextual analysis of DROP commands
  - Legitimate test table operations allowed
  - Production table CASCADE operations blocked

## **Part 2: Nuanced contextual situations: permissioned capabilities, dynamic procedures**

EQTY provides context-aware evaluation through multiple mechanisms: advanced contextual understanding, risk acceptance allowances, and/or agentic capabilities in emerging scenarios where you deploy AI to monitor and review other AI.

The most direct answer to the truly unknown or advanced edge cases requires the last mentioned intelligent capabilities:

### **Agentic Oversight**

EQTY enables multi-agent assurance patterns where capable reasoning models review an executing-model’s logs and actions, with EQTY providing the integration layer for intelligent oversigh & policy enforcement. AI is the guardrail for AI in this case \- “Stop\! You didn’t actually do that\! or… Stop, you forgot about rule X” … “Stop\! That will delete an important file and we don’t have a backup\!”

### **Self-Agentic Guardrailing**

Similar to Test-Driven Development, responsible agents can use EQTY's tooling to create guardrails for themselves as they determine dynamic procedures on the fly.

### **Advanced, non-agentic, risk assurance**

#### **1\. Context-Aware Conditions**

- Policy enforcement allows for arbitrary execution of evaluation.
- This includes the ability to account for uncertainty, and address such through a risk tolerance lens.
- “I need my agent to be able to delete files, but it should never delete the mission critical files” …. While there are simple assurances to account for here (“dont delete any of the critical tables” \-\> real guardrails), this type of scenario is addressed by integrated context-gathering and risk thresholding for non-deterministic scenarios.

```
- name: "Smart SQL Drop Protection"
  conditions:
    - pattern: "DROP|TRUNCATE"
    - check: "check-db-risk.sh"  # Analyzes relationships, data volume (no risk: test, high risk: prod table with 165,000 rows of data)
  action: block_if_any_risk
```

#### **2\. Multi-Factor Context Awareness and Policy Evaluation**

- **WHO**: is the agent (as well as its verification, its compliance)
- **WHO:** is the person, or organization, the agent acting on behalf of
- **WHAT**: is the behavioral action, and result
- **WHERE**: is this happening
- **WHEN**: is the agent doing this, what will happen next
