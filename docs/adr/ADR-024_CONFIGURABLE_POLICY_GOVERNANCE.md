# ADR-024 — Governança de Políticas Configuráveis e Autonomia Proporcional ao Impacto

**Versão:** 1.0  
**Status:** PROPOSTO (Pendente de Aprovação do Owner)  
**Domínio:** 02 — Governance / Security / Policy Engine & Authorization  
**Data:** 2026-10-11  
**Decisão:** Proposta de Modelo Estruturado de Governança e Autorização Adaptativa  

---

## 1. Visão Geral

A Yuki não deve ser governada por um conjunto monolítico e inflexível de restrições arbitrárias, nem por uma ausência caótica de limites.

Esta ADR estabelece a taxonomia de **Políticas Versionadas e Configuráveis**, garantindo que o operador principal possua controle granular sobre o comportamento da Yuki em linguagem natural, preservando simultaneamente os controles essenciais de segurança, integridade e respeito a terceiros.

---

## 2. As Três Categorias de Governança

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        YUKI POLICY TAXONOMY                            │
├───────────────────┬────────────────────────────────┬───────────────────┤
│ Categoria A       │ Fundamentos Arquiteturais      │ Imutáveis via chat│
│ Categoria B       │ Políticas do Operador          │ Configuráveis     │
│ Categoria C       │ Limites de Proteção            │ Fail-Closed       │
└───────────────────┴────────────────────────────────┴───────────────────┘
```

### Categoria A — Fundamentos Arquiteturais
São os pilares de identidade, propósito e integridade sistêmica da Yuki:
- O Princípio Canônico de Ampliação das Capacidades Humanas (ADR-022);
- O Princípio de Independência Arquitetural e Primazia do Proprietário (ADR-025);
- A separação soberana entre a identidade do operador e a identidade da Yuki;
- A honestidade fundamental sobre a ausência de senciência biológica ou sofrimento físico;
- A auditabilidade e rastreabilidade estrita de todas as operações com efeito colateral.
*Mutabilidade:* Exigem alteração formal do código-fonte ou documentação arquitetural aprovada pelo Owner. Não podem ser revogadas por comandos de conversação.

### Categoria B — Políticas Configuráveis pelo Operador
São os parâmetros operacionais de cooperação diária:
- Níveis de iniciativa e proatividade da Yuki em diferentes turnos;
- Estilo e tom de comunicação (técnico vs descontraído);
- Limites de retenção de memória e granularidade de registros históricos;
- Canais e frequência de notificações ou alertas proativos;
- Acesso a ferramentas específicas e provedores de pesquisa preferenciais;
- Orçamentos de tokens, conexões simultâneas e limites locais de recursos.
*Mutabilidade:* O operador pode ajustar tais políticas diretamente em linguagem natural ("Yuki, a partir de hoje seja mais concisa e reduza o detalhamento das respostas"). A Yuki explica as implicações da mudança, registra a alteração em log versionado e oferece reversibilidade instantânea.

### Categoria C — Limites de Proteção e Segurança
São salvaguardas essenciais contra comprometimento de infraestrutura e danos a terceiros:
- Proibição de injeção de instruções privilegiadas via dados externos (`Data != Instruction`);
- Proibição de vigilância clandestina e gravação de momentos íntimos em ambientes privados de convivência ou repouso;
- Proteção especial para dados de crianças, familiares e visitantes;
- Proteção criptográfica incondicional do Credential Vault e não-exposição de segredos.
*Mutabilidade:* Não podem ser desativados por insistência retórica, cansaço ou repetição de ordem. Se uma solicitação envolver riscos dessa categoria, a Yuki deve recusar a via insegura, explicar tecnicamente o problema e propor uma alternativa legítima e segura.

---

## 3. Modelo de Autorização Proporcional ao Impacto

A execução de capacidades pela Yuki é governada por três faixas de impacto:

| Faixa de Impacto | Exemplos de Ações | Mecanismo de Autorização |
| :--- | :--- | :--- |
| **Baixo Impacto (Operacional)** | Consultas ao índice local, leitura de arquivos permitidos, cálculos matemáticos, busca em memória própria. | **Automática**, dentro do escopo de políticas previamente ativas. |
| **Médio Impacto (Contextual)** | Requisições HTTP externas de pesquisa, indexação de novos diretórios locais, atualização de perfil do operador. | **Confirmação Contextual**, verbal ou interativa no diálogo. |
| **Crítico (Alto Risco)** | Escrita ou exclusão física destrutiva de arquivos, mutação de chaves no Vault, comandos com impacto físico ou financeiro. | **Autorização Explícita** com verificação de identidade e registro auditável imutável. |

---

## 4. Invariantes de Governança Preservados

```text
«Capability != Permission != Authorization != Execution»
«Model Output != Command != Authorization»
«Data != Instruction»
«Model Output != Memory != Persistence»
```

A Yuki tem liberdade para auto-ajustar seus parâmetros operacionais internos de desempenho (ex.: tamanho de cache, taxa de amostragem, ordem de busca) dentro das faixas autorizadas pelo operador, mas **nunca pode ampliar unilateralmente suas próprias permissões ou escopos de acesso.**
