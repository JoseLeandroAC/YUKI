# Proposta Arquitetural — Yuki Ambient Intelligence & Security

**Versão:** 1.0  
**Status:** PROPOSTA ARQUITETURAL (Futura — Não implementada prematuramente)  
**Domínio:** 16 — Ambient Intelligence / Physical Sensors / Domestic Safety & Privacy  
**Data:** 2026-10-11  
**Autor:** Proposta para análise e aprovação do Owner  

---

## 1. Visão Geral e Finalidade

O subsistema **Yuki Ambient Intelligence & Security** é concebido como um módulo autônomo, porém coordenado pela plataforma central da Yuki, destinado a prover:
- Consciência situacional do ambiente físico;
- Segurança preventiva residencial;
- Monitoramento de eventos e detecção de anomalias domésticas;
- Reconhecimento de rotinas e padrões consentidos pelos moradores;
- Assistência contextual discreta e não-intrusiva.

Este subsistema é uma extensão física e sensorial das capacidades da Yuki, projetado para operar com **permissões, contratos, políticas e armazenamento de dados estritamente segregados** do núcleo cognitivo geral.

---

## 2. Separação de Identidade e Autenticação do Operador

Diferente de sistemas biométricos invasivos:
- **Não há exigência de microfone ou câmera continuamente ligados para identificar o operador.**
- A arquitetura separa com clareza:
  1. *Identidade Lógica do Operador* (quem ele é no sistema);
  2. *Autenticação de Sessão* (chaves, senhas, tokens efêmeros de sessão);
  3. *Confiança do Dispositivo* (chaves de hardware, TLS mútuo, dispositivo pareado);
  4. *Autorização da Operação* (proporcional ao impacto da ação);
  5. *Escopo de Acesso a Dados* (isolamento por proprietário `owner_id`).
- Reconhecimento facial ou de voz é **estritamente opcional** e nunca constitui autorização suficiente isolada para operações críticas destrutivas.

---

## 3. Capacidades Futuras Propostas

1. **Integração Sensorial Segura:** Sensores perimetrais, câmeras de acesso externo, sensores de presença, temperatura, fumaça e consumo energético.
2. **Detecção de Chegadas e Saídas:** Notificação situacional de entrada/saída de veículos ou familiares no perímetro externo.
3. **Identificação de Anomalias:** Portões esquecidos abertos, vazamento de água, movimentação perimetral em horários anômalos.
4. **Histórico Consultável e Resumos:** Capacidade de responder consultas retrospectivas sobre acontecimentos ("Yuki, que horas o entregador passou ontem?").
5. **Automação Doméstica Contextual:** Integração com iluminação, climatização e dispositivos inteligentes com modos operacionais pré-autorizados.
6. **Modos de Privacidade por Ambiente:** Regras rigorosas de desligamento de sensores ou ofuscação automática por horário, presença ou localização.

---

## 4. O Princípio da Discrição Contextual

### 4.1 Exemplo Prático de Operação
> *Cenário:* Uma câmera externa de perímetro detecta que integrantes da família ou parentes chegaram inesperadamente ao portão. O operador principal está em uma reunião ou momento pessoal com preferência configurada de não-interrupção.
>
> *Comportamento da Yuki:*
> - A Yuki identifica o evento externo relevante através de telemetria perimetral;
> - Envia um aviso discreto e neutro ao responsável por canal apropriado (ex.: vibração em relógio, notificação no celular ou aviso sutil em tela secundária);
> - O aviso informa com objetividade o fato externo ("Visitantes chegaram ao portão principal");
> - **Invariante de Privacidade:** Não é necessário capturar áudio, filmar ou analisar o ambiente íntimo em que o operador se encontra para cumprir essa função com eficácia e respeito.

---

## 5. Matriz de Classificação de Informações Ambientais

O sistema distingue obrigatoriamente quatro categorias de acontecimentos:

| Categoria | Descrição | Tratamento & Retenção |
| :--- | :--- | :--- |
| **1. Segurança Operacional** | Alarmes perimetrais, falhas de sensor, invasões, fogo, anomalias. | Retenção durável auditável com log criptográfico. |
| **2. Acontecimentos Domésticos Relevantes** | Entregas recebidas, visitas externas, partidas e chegadas de veículos. | Resumo consolidado com retenção configurável (ex.: 30 dias). |
| **3. Rotinas Pessoais Consentidas** | Horário de despertar, saída para trabalho, preferências de iluminação. | Padrões estatísticos agregados sem armazenamento de vídeo bruto contínuo. |
| **4. Atividades Íntimas e Privadas** | Momentos íntimos, repouso, higiene pessoal, conversas familiares íntimas. | **PROIBIÇÃO DE GRAVAÇÃO E ARMAZENAMENTO POR PADRÃO.** |

---

## 6. Salvaguardas e Políticas de Privacidade

1. **Processamento Efêmero na Borda:** A análise de visão computacional ocorre localmente em tempo real; frames de vídeo brutos de ambientes internos são descartados após a extração de eventos operacionais, a menos que haja gravação deliberada e autorizada pelo operador.
2. **Proibição Absoluta de Monitoramento Clandestino:** Sensores em quartos, banheiros e áreas de intimidade têm modo cego (blind mode) ou são vedados pelo sistema.
3. **Proteção Reforçada a Crianças:** Eventos envolvendo menores de idade recebem criptografia reforçada, ausência de indexação externa e restrição estrita de visualização.
4. **Escopos Distintos entre Moradores e Visitantes:** Cada membro da família e visitante possui seu próprio escopo. O operador principal administra a infraestrutura física de segurança, mas a Yuki não atua como mecanismo de espionagem ou vigilância doméstica contra os direitos fundamentais de outros moradores.
5. **Transparência Situacional:** Sinalização clara sobre quais sensores e áreas estão ativas a qualquer momento.

---

## 7. Próximos Passos

Esta proposta será submetida à revisão e aprovação do Owner. Nenhuma implementação de código de drivers de câmeras ou sensores ambientais será iniciada sem a prévia especificação e autorização em marco dedicado futuro.
