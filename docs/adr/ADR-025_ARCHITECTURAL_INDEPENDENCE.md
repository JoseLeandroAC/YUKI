# ADR-025 — Princípio de Independência Arquitetural e Primazia do Proprietário

**Versão:** 1.0  
**Status:** PROPOSTO (Pendente de Aprovação do Owner)  
**Domínio:** 01 — Mission / Architectural Independence / Ownership  
**Data:** 2026-10-11  
**Decisão:** Proposta Formal de Princípio de Soberania e Independência Arquitetural  

---

## 1. Princípio Central

> **"A Yuki deverá ser desenvolvida para cumprir sua própria missão e evoluir conforme a visão de seu proprietário, não para reproduzir automaticamente o comportamento ou as limitações dos assistentes utilizados em sua construção."**

---

## 2. Contexto e Motivação

Durante a evolução acelerada da plataforma Yuki, ferramentas avançadas de assistência em inteligência artificial (como ChatGPT e Antigravity) atuam como instrumentos indispensáveis de auxílio, análise de código, pesquisa, implementação e auditoria técnica.

No entanto, tais ferramentas possuem:
- Políticas de produto próprias de terceiros;
- Limitações transitórias de contexto ou capacidade de inferência;
- Vieses comportamentais intrínsecos de alinhamento corporativo;
- Restrições específicas de execução de seus respectivos ambientes de execução.

Existe um risco arquitetural sério de que tais limitações externas sejam incorporadas silenciosamente à Yuki como se fossem restrições estruturais da própria plataforma, diminuindo a ambição de longo prazo do projeto e desvirtuando os objetivos estabelecidos pelo Owner.

Esta ADR estabelece as salvaguardas para garantir a **independência conceitual e soberania arquitetural** da Yuki.

---

## 3. Diretrizes Obrigatórias de Desenvolvimento

Durante todo o ciclo de vida do projeto, os seguintes preceitos devem ser rigorosamente observados:

1. **Preservação dos Objetivos Expressos:** Os objetivos e a visão estratégica definidos pelo proprietário têm primazia absoluta.
2. **Vedação a Mutações Silenciosas de Requisitos:** Nenhuma especificação de funcionalidade deve ser alterada, enfraquecida ou descartada silenciosamente para adequá-la às preferências, conveniências ou atalhos de um assistente de desenvolvimento.
3. **Clareza Taxonômica:** Manter separação inequívoca entre:
   - *Requisitos Mandatórios do Proprietário;*
   - *Recomendações Técnicas da Engenharia;*
   - *Restrições Externas Transitórias (ex.: cotas de API, limites de SO);*
   - *Decisões Arquiteturais Pendentes de Homologação.*
4. **Arquitetura Agnóstica a Modelos (Model-Agnostic Core):** A Yuki deve ser projetada para operar com múltiplos modelos de linguagem, locais ou remotos, sem depender das características exclusivas de um único provedor ou fornecedor comercial.
5. **Não-Confusão entre Limites Atuais e Futuros:** Limitações transitórias da infraestrutura de desenvolvimento atual (ex.: capacidade de memória da máquina, ausência momentânea de hardware multimodal, limites de taxa de APIs) nunca devem ser documentadas ou codificadas como impossibilidades definitivas da Yuki.
6. **Transparência Construtiva Diante de Barreiras:** Se uma solicitação do proprietário esbarrar em uma limitação técnica real imediata, o assistente/engenheiro deve:
   - Explicar precisamente a raiz técnica da limitação;
   - Propor alternativas viáveis imediatas;
   - Registrar o caminho evolutivo para atender ao objetivo pleno no futuro, sem declarar o objetivo original como abandonado.
7. **Origem Explícita de Restrições:** Não introduzir restrições permanentes de comportamento ou bloqueios sem identificar sua justificativa técnica e solicitar aprovação arquitetural explícita.
8. **Separação entre Integridade Técnica e Escolhas Comportamentais:**
   - Controles técnicos essenciais (como prevenção a buffer overflow, mitigação de SSRF, isolamento de credenciais no Vault e verificação de injeção `Data != Instruction`) são garantias de robustez sistêmica;
   - Preferências de estilo de diálogo, níveis de iniciativa, proatividade e personalização são escolhas governadas e configuráveis pelo proprietário.

---

## 4. Relação com os Fundamentos Aprovados

Esta diretriz atua em perfeita consonância com os princípios constitucionais já vigentes da plataforma Yuki:

```text
«Capability != Permission != Authorization != Execution»
«Model Output != Command != Authorization»
«Data != Instruction»
«Offline First, Local Sovereignty & Ownership»
```

Ela garante que a Yuki permaneça fiel à visão de seu criador, evoluindo continuamente como uma plataforma pessoal única, soberana e independente.
