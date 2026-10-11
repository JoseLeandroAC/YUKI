# ADR-023 — Arquitetura de Memória Cognitiva, Associativa e Expansível

**Versão:** 1.0  
**Status:** PROPOSTO (Pendente de Aprovação do Owner)  
**Domínio:** 04 — Memory / Cognitive Architecture / Associative Graphs  
**Data:** 2026-10-11  
**Decisão:** Proposta Arquitetural de Longo Prazo para a Evolução da Memória da Yuki  

---

## 1. Visão Geral e Contexto

No Marco 7 e no Marco 7.1, a Yuki estabeleceu a **Memory v1**: uma fundação durável sobre SQLite e FTS5 com suporte a tipologia canônica básica (`Instruction`, `Preference`, `Fact`, `ProjectContext`, `Episode`), índices relacionais com escopo de proprietário (`owner_id`), sanitização fail-closed de credenciais e isolamento estrito de injeção (`Data != Instruction`).

Entretanto, uma inteligência pessoal duradoura e multimodal não pode ter sua capacidade cognitiva restrita a um modelo transacional simples de tabela plana com limite estático de registros.

Esta ADR projeta a transição futura da **Memory v1** para um **Sistema de Memória Cognitiva e Associativa de Multi-Camadas**.

---

## 2. As Camadas Cognitivas da Memória Yuki

A arquitetura cognitiva divide a memória em subsistemas especializados com ciclos de vida, retenção e taxas de decaimento próprios:

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        YUKI COGNITIVE MEMORY                           │
├───────────────────┬────────────────────────────────┬───────────────────┤
│ 1. Working Memory │ Sessão ativa / Scratchpad      │ Alta volatilidade │
│ 2. Episodic       │ Diário de eventos & diálogos   │ Temporal sequencial│
│ 3. Semantic       │ Conceitos, teses e aprendizados│ Conceitual / Fatos│
│ 4. Personal/Auto  │ Preferências, valores e rotinas│ Alta estabilidade │
│ 5. Project/Goals  │ Decisões, ADRs e roadmaps      │ Estruturado       │
│ 6. Associative    │ Grafo de entidades e conexões  │ Topologia de nós  │
└───────────────────┴────────────────────────────────┴───────────────────┘
```

### 2.1 Memória de Trabalho (Working Memory)
- Mantém o estado dinâmico do diálogo, objetivos imediatos da tarefa e artefatos intermediários em uso.
- Limpa ou consolida ao término da sessão interativa.

### 2.2 Memória Episódica (Episodic Memory)
- Grava ocorrências temporais ordenadas ("O que aconteceu, quando e em que sequência").
- Permite responder perguntas sobre histórico operacional, incidentes passados e evolução de conversas anteriores.

### 2.3 Memória Semântica (Semantic Memory)
- Conhecimento consolidado desacoplado do tempo ("Como funciona o FTS5", "Quais são as regras do PBKDF2 na Yuki").
- Extraída de documentações, pesquisas confirmadas e conclusões validadas.

### 2.4 Memória Pessoal e Autobiográfica (Personal Memory)
- Hábitos, estilo de comunicação, preferências de trabalho, timezone e peculiaridades autorizadas do operador.
- Mantida sob estrita governança de privacidade e isolamento por `owner_id`.

### 2.5 Memória de Projetos, Decisões e Objetivos (Project & Goal Memory)
- Grafos de dependência de projetos, marcos em andamento, riscos mapeados e histórico de decisões arquiteturais tomadas.

### 2.6 Memória Associativa (Knowledge & Entity Graph)
- Ligações entre entidades (Pessoas ↔ Projetos ↔ Tecnologias ↔ Eventos).
- Permite à Yuki inferir contexto cruzado (ex.: relacionar uma menção casual a um projeto ao histórico de uma ferramenta específica pesquisada semanas antes).

---

## 3. Consolidação de Experiências e Síntese Cognitiva

A memória não cresce apenas por acumulação de logs brutos; ela amadurece por **consolidação**:

1. **Processo de Consolidação Noturna / Ociosa:**
   - Transforma centenas de interações diárias detalhadas em sínteses compactas e densas de significado;
   - Registros originais podem ser arquivados em armazenamento frio ou expirar conforme política de retenção;
   - As sínteses preservam a proveniência: o sistema mantém a distinção entre fatos originais declarados, sínteses consolidadas e deduções inferidas.
2. **Importância Não-Linear:**
   - A relevância de uma memória não é determinada apenas pela frequência com que algo é repetido;
   - Uma experiência rara, uma descoberta técnica crítica ou uma instrução de segurança enfática do operador possui peso de importância maior do que semanas de conversas triviais.

---

## 4. Limites Dinâmicos, Paginação e Orçamento Adaptativo

A plataforma distingue três grandezas frequentemente confundidas:
1. **Capacidade Total de Armazenamento:** Virtualmente ilimitada no disco durável local.
2. **Quantidade de Resultados Consultados pelo Retriever:** Escalável via paginação progressiva (`LIMIT`, `OFFSET` dinâmicos ou cursores por timestamp/rank).
3. **Orçamento Alocado no Contexto do Modelo de Linguagem:** Controlado dinamicamente via `PersonalContextConfig` e `effective_char_limit()`, respeitando a proporção da janela de contexto do modelo utilizado.

A Yuki nunca trata limites transitórios de uma sessão como impossibilidades permanentes do sistema. Ela realiza recuperação progressiva sob demanda.

---

## 5. Esquecimento Seletivo e Higiene de Privacidade

- **Direito ao Esquecimento Efetivo:** Comandos explícitos de revogação ou esquecimento (`yuki memory delete/forget`) removem fisicamente dados de tabelas relacionais, índices FTS5 e grafos associativos.
- **Isolamento de Espaços:** Ambientes de terceiros (visitantes, hóspedes, integrações externas) possuem escopos lógicos rigorosamente segregados (`owner_id`), impossibilitando vazamento para o contexto primário do operador.

---

## 6. Status e Roteiro de Implementação

| Componente | Status Atual | Próximo Marco |
| :--- | :--- | :--- |
| Armazenamento Relacional + FTS5 | **Implementado & Testado (Marco 7/7.1)** | Manutenção contínua |
| Paginação SQL & Isolamento de Dono | **Implementado & Testado (Marco 7.1)** | Produção |
| Grafo Associativo de Entidades | **Proposto (ADR-023)** | Marco 8+ |
| Consolidação de Sínteses Ociosas | **Proposto (ADR-023)** | Marco 8+ |
| Memória Vetorial Híbrida (Embeddings) | **Proposto (ADR-021/023)** | Marco 8+ |
