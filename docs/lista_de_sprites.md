# Catálogo de Sprites Necessários - Bestigard

Este documento lista todos os assets visuais em 2D/isométrico necessários para o jogo, com identificadores padronizados (usados nos atlas e no código), dimensões recomendadas e descrições visuais detalhadas.

> **Padrão Gráfico:** Arte 2D estilizada ou pixel art limpa de alta densidade.
> **Projeção de Terrenos/Estruturas:** Isométrica 2:1 (ex: largura 128px, altura 64px para base de tiles).
> **Formato de Entrega:** PNG com canal alfa transparente (32-bit RGBA).

---

## 1. Criaturas & Ovos (Bestiário)
*Dimensões recomendadas:* 
- **Ovos:** 64x64 px
- **Baby:** 64x64 px ou 96x96 px
- **Juvenile:** 128x128 px
- **Adult:** 160x160 px ou 192x192 px

### 1.1 `pyro_drake` (Dragão Pírico - Fogo | Comum)
- `egg_pyro_drake`: Ovo de casca carmesim vulcânica com veios brilhantes de magma incandescente.
- `creature_pyro_drake_baby`: Pequeno lagarto alado com olhinhos curiosos, escamas avermelhadas e chama tênue na ponta da cauda.
- `creature_pyro_drake_juvenile`: Dragão jovem com chifres pontiagudos, postura ágil e asas membranosas tingidas em gradiente de fogo.
- `creature_pyro_drake_adult`: Majestoso dragão elemental de asas abertas, escamas de obsidiana e rubi, crista flamejante e olhar imponente.

### 1.2 `aqua_serpent` (Serpente das Marés - Água | Comum)
- `egg_aqua_serpent`: Ovo azul-turquesa translúcido com padrões ondulados de água corrente e gotas de orvalho.
- `creature_aqua_serpent_baby`: Filhote de serpente aquática enrolado, barbatanas dorsais translúcidas e expressão dócil.
- `creature_aqua_serpent_juvenile`: Serpente marinha esguia com barbatanas onduladas, crista de coral suave e escamas de safira.
- `creature_aqua_serpent_adult`: Leviatã elegante rodeado por anéis de água viva flutuante, escamas iridescentes e crista coralina majestosa.

### 1.3 `gale_hawk` (Falcão dos Ventos - Ar | Comum)
- `egg_gale_hawk`: Ovo branco perolado e aerodinâmico com redemoinhos de brisa suave gravados na casca.
- `creature_gale_hawk_baby`: Pintinho fofo com penugem azul-celeste esvoaçante, bico curto e asinhas hiperativas.
- `creature_gale_hawk_juvenile`: Ave de rapina ágil com penas afiadas nas asas e cauda bifurcada que corta o vento.
- `creature_gale_hawk_adult`: Grande predador celeste de peito estufado, envergadura imponente, asas com pontas de tempestade e olhos dourados.

### 1.4 `stone_golem` (Golem Rochoso - Terra | Comum)
- `egg_stone_golem`: Geodo arredondado de rocha cinzenta coberto de musgo com fissuras revelando minério dourado no interior.
- `creature_stone_golem_baby`: Pequeno bloco de granito animado com seixos como pés, bracinhos curtos e olhos de esmeralda reluzente.
- `creature_stone_golem_juvenile`: Construto rochoso mais musculoso, placas de quartzo nos ombros e vinhas entre as fendas de pedra.
- `creature_stone_golem_adult`: Colosso de rochas flutuantes e basalto maciço, com runas esculpidas no peito, barba de musgo e punhos gigantes.

### 1.5 `thunder_striker` (Centelha Trovejante - Raio | Raro)
- `egg_thunder_striker`: Ovo amarelo-âmbar com padrão de ziguezague elétrico e pequenas faíscas estáticas na casca.
- `creature_thunder_striker_baby`: Felino/lagartinho miúdo com orelhas pontiagudas, bigodes condutores e cauda em formato de relâmpago.
- `creature_thunder_striker_juvenile`: Guepardo elétrico ágil com listras luminosas amarelas e azul-elétrico ao longo do corpo.
- `creature_thunder_striker_adult`: Fera tempestuosa lendária cercada por uma aura permanente de plasma e arcos voltaicos estalando nas patas.

### 1.6 `solar_phoenix` (Fênix Solar - Luz | Épico)
- `egg_solar_phoenix`: Ovo dourado espelhado que irradia luz própria suave e parece pulsar calor confortante.
- `creature_solar_phoenix_baby`: Pintinho dourado cintilante com pequenas penas luminosas parecendo chamas de vela branca.
- `creature_solar_phoenix_baby`: Pássaro esbelto de penas de luz pura, cauda longa em fitas douradas e bico de marfim.
- `creature_solar_phoenix_adult`: Pássaro solar sagrado de grande porte com asas flamejantes brancas e douradas, coroa solar e rastro radiante.

### 1.7 `void_stalker` (Rastejante do Abismo - Sombra | Lendário)
- `egg_void_stalker`: Esfera de escuridão pura que absorve reflexos de luz ao redor, envolta em névoa violeta escura.
- `creature_void_stalker_baby`: Pequena massa espectral escura com dois olhos violetas brilhantes e pequenas patinhas sombrias.
- `creature_void_stalker_juvenile`: Predador quadrúpede sombrio com corpo semitransparente feito de fumaça cósmica e presas brilhantes.
- `creature_void_stalker_adult`: Monstruosidade cósmica com múltiplas caudas espectrais flutuantes, chifres de vácuo estelar e garras dimensionais.

### 1.8 `volcano_beast` (Besta Vulcânica - Fogo + Terra | Raro)
- `egg_volcano_beast`: Ovo de basalto escuro com crosta rachada vazando magma viscoso incandescente.
- `creature_volcano_beast_baby`: Tartaruguinha de carapaça rochosa quente com uma fumaça fina saindo de um furinho dorsal.
- `creature_volcano_beast_juvenile`: Quadrúpede couraçado com placas tectônicas sobrepostas e veios de lava líquida avermelhada.
- `creature_volcano_beast_adult`: Tartaruga-vulcão colossal carregando uma mini-cratera ativa no dorso com fumaça e pequenas rochas em erupção.

### 1.9 `storm_bringer` (Portador da Tormenta - Água + Ar + Raio | Lendário)
- `egg_storm_bringer`: Ovo cinza-tempestade que lembra uma nuvem cumulonimbus compacta, com gotículas de chuva e faíscas.
- `creature_storm_bringer_baby`: Pequeno dragão aquático-aéreo feito de nuvem macia, gotinhas flutuando ao redor e chifrinho de raio.
- `creature_storm_bringer_juvenile`: Serpente alada feita de nevoeiro tempestuoso condensado, com asas translúcidas e olhos cintilantes.
- `creature_storm_bringer_adult`: Grande dragão dos mares tempestuosos com juba de nuvens carregadas, relâmpagos internos e espirais de vento cortante.

### 1.10 `mud_crawler` (Rastejador da Lama - Água + Terra | Incomum)
- `egg_mud_crawler`: Ovo de argila úmida salpicado de musgo aquático e pequenas folhas de lótus.
- `creature_mud_crawler_baby`: Anfíbio gordinho e sorridente coberto por lodo refrescante e pequenas nadadeiras palmadas.
- `creature_mud_crawler_juvenile`: Salamandra robusta com casco terroso, plantas pantanosas crescendo no lombo e cauda achatada.
- `creature_mud_crawler_adult`: Besta gigante do pântano com couraça de terra endurecida, juncos floridos nas costas e patas imensas.

### 1.11 `eclipse_chimera` (Quimera do Eclipse - Luz + Sombra | Mítico)
- `egg_eclipse_chimera`: Ovo perfeitamente dividido na vertical: metade branca com brilho solar e metade negra com auréola lunar roxa.
- `creature_eclipse_chimera_baby`: Criatura de duas cores (metade branca / metade negra) com asas contrastantes e olhar tímido e místico.
- `creature_eclipse_chimera_juvenile`: Grifo quimérico com asa direita de penas solares douradas e asa esquerda de pura névoa espectral negra.
- `creature_eclipse_chimera_adult`: Divindade cósmica do eclipse com coroa de luz celestial, cauda de nébula escura e asas monumentais em equilíbrio.

---

## 2. Habitats Elementais (Isométricos)
*Dimensões recomendadas:* 256x256 px ou 192x192 px (base isométrica 2x2 ou 3x3 tiles).

### 2.1 Habitat de Fogo (`fire`)
- `habitat_fire_t1`: Clareira de terra queimada com fogueira central sobre pedras de carvão incandescente.
- `habitat_fire_t2`: Lago de magma circular cercado por colunas de basalto negro e faíscas subindo.
- `habitat_fire_t3`: Fortaleza vulcânica ardente com pequenos rios de lava, gárgulas flamejantes e altar de chamas.

### 2.2 Habitat de Água (`water`)
- `habitat_water_t1`: Lagoa circular límpida com pedras de rio lisas na borda e pequenas vitórias-régias.
- `habitat_water_t2`: Baía de corais multicoloridos com mini cascata d'água fresca caindo de rochas cristalinas.
- `habitat_water_t3`: Santuário marinho majestoso com gêiser central pulsante, colunas de conchas e água turquesa bioluminescente.

### 2.3 Habitat de Ar (`air`)
- `habitat_air_t1`: Platô rochoso elevado com cata-ventos de madeira e fitas coloridas que tremulam com a brisa.
- `habitat_air_t2`: Pedaço de ilha flutuante ancorada por correntes leves, envolta em brumas e redemoinhos suaves.
- `habitat_air_t3`: Templo nos céus com colunas esguias de mármore eólico, harpas eólicas que tocam com o vento e chão de nuvem compacta.

### 2.4 Habitat de Terra (`earth`)
- `habitat_earth_t1`: Montes de terra fértil com pedregulhos naturais, grama rasteira e tronco de árvore caído.
- `habitat_earth_t2`: Caverna aberta de pedreira com grandes geodos de quartzo e formações de estalagmites.
- `habitat_earth_t3`: Fortaleza megalítica ancestral com monólitos rúnicos erguidos, raízes gigantescas entrelaçadas e gemas terrestres.

### 2.5 Habitat de Raio (`lightning`)
- `habitat_lightning_t1`: Torre rústica de madeira com para-raios de cobre e pequenas bobinas receptoras de eletricidade estática.
- `habitat_lightning_t2`: Platô com pilares metálicos energizados disparando arcos de eletricidade constante entre si.
- `habitat_lightning_t3`: Estação de tempestade com bobina de Tesla gigantesca central, anéis magnéticos flutuantes e chão ionizado.

### 2.6 Habitat de Luz (`light`)
- `habitat_light_t1`: Clareira de mármore branco com pedestal de prisma de cristal refletindo raios de sol em arco-íris.
- `habitat_light_t2`: Jardim solar pavimentado em ouro e mármore, cercado por girassóis mágicos que emitem brilho dourado.
- `habitat_light_t3`: Catedral celestial aberta com cúpula de raios solares condensados, obeliscos de cristal puro e fontes douradas.

### 2.7 Habitat de Sombra (`shadow`)
- `habitat_shadow_t1`: Cova de pedra escura sombreada por folhagens púrpuras e leve névoa cinzenta rastejante.
- `habitat_shadow_t2`: Círculo ritualístico de obsidiana polida com velas de chamas violetas e espelhos sombrios reflexivos.
- `habitat_shadow_t3`: Fenda dimensional abissal no chão emanando névoa negra espessa, com cristais de ametista corrompida e portais menores.

---

## 3. Construções, Fazendas & Ninhos
*Dimensões recomendadas:* 192x192 px ou 160x160 px.

- `building_hatchery_empty`: Ninho de incubação feito de ramos entrelaçados e forrado com folhas macias e penas (vazio).
- `building_hatchery_incubating`: Ninho contendo ovo aninhado sob luz suave de aquecedor mágico.
- `building_hatchery_ready`: Ninho com ovo brilhando intensamente e pequenas rachaduras com faíscas prontas para quebrar.
- `building_breeding_sanctuary_idle`: Altar de cruzamento esculpido em pedra mágica com dois pedestais laterais e runas apagadas.
- `building_breeding_sanctuary_active`: Altar de cruzamento com energia mágica brilhante em espiral unindo os dois lados em um casulo de luz.
- `building_farm_empty`: Canteiro de terra arada fértil e fofa, dividido em sulcos para plantio (2x2 tiles).
- `building_farm_growing`: Canteiro com brotos verdes pequenos e irrigação mágica em andamento.
- `building_farm_ready`: Canteiro com plantações exuberantes e brilhantes, repletas de frutos prontos para colheita.

---

## 4. Culturas Agrícolas (Ícones de Comida)
*Dimensões recomendadas:* 64x64 px.

- `crop_wild_berry`: Cacho de frutas vermelhas silvestres brilhantes com folhinhas verdes.
- `crop_sun_wheat`: Feixe de trigo dourado brilhante como sol da manhã com laço rústico.
- `crop_lunar_shroom`: Cogumelo azul-noite fosforescente com pintas prateadas e esporos luminosos.
- `crop_star_root`: Raiz mágica em forma de estrela com brilho cósmico amarelo e violeta.

---

## 5. Terreno & Ilha Isométrica
*Dimensões recomendadas:* 128x64 px (tile padrão 2:1) e peças especiais de borda.

- `tile_island_grass_01`: Grama verde esmeralda uniforme com textura suave e limpa.
- `tile_island_grass_02`: Grama verde com florzinhas silvestres amarelas e brancas.
- `tile_island_dirt`: Terra batida amarronzada para caminhos e áreas de construção.
- `tile_island_stone`: Pavimento de pedras rústicas hexagonais / paralelepípedos.
- `tile_island_water`: Água límpida e cristalina para lagoas ou bordas de ilha.
- `tile_island_cliff_edge_n`: Borda de penhasco norte mostrando rocha nua e raízes penduradas.
- `tile_island_cliff_edge_s`: Borda de penhasco sul mostrando o fundo rochoso da ilha flutuando no céu.
- `tile_island_cliff_edge_e`: Borda de penhasco leste.
- `tile_island_cliff_edge_w`: Borda de penhasco oeste.
- `decal_rock_small`: Pequeno pedregulho decorativo para o solo.
- `decal_bush`: Arbusto verde esmeralda arredondado.
- `decal_tree_mystic`: Árvore frondosa com copa mágica esvoaçante.

---

## 6. Interface de Usuário (UI) & HUD
*Dimensões recomendadas:* 64x64 px para ícones e 128x48 px para botões/molduras.

### 6.1 Recursos
- `icon_gold_coin`: Moeda dourada grossa com relevo de asa de dragão e brilho especular.
- `icon_food_apple`: Maçã ou cesta transbordando alimentos deliciosos e reluzentes.
- `icon_gem_crystal`: Diamante lapidado azul-celeste ou gema roxa luminosa.

### 6.2 Elementos (Ícones de Atributo)
- `icon_element_fire`: Chama viva vermelha e laranja estilizada em círculo.
- `icon_element_water`: Gota d'água azul límpida com espiral interior.
- `icon_element_air`: Três redemoinhos de vento em curva elegante verde-água/ciano.
- `icon_element_earth`: Pedra angular facetada com folhas marrom e verde musgo.
- `icon_element_lightning`: Raio agudo amarelo com contorno dourado intenso.
- `icon_element_light`: Sol de oito pontas branco e dourado irradiando claridade.
- `icon_element_shadow`: Crescente lunar envolto em névoa espectral violeta e negra.

### 6.3 Botões & Menus
- `btn_shop`: Ícone de barraca ou sacola de compras mágica.
- `btn_bestiary`: Livro de capa de couro com um olho de criatura ou gema na capa.
- `btn_breeding`: Dois corações elementais ou esferas de luz entrelaçadas.
- `btn_arena`: Duas espadas ou garras cruzadas com faíscas.
- `btn_settings`: Engrenagem mecânica de latão dourado.
- `ui_panel_frame`: Borda de janela estilizada (9-slice) em tom azul-escuro marinho com detalhes dourados.
- `ui_health_bar_fill`: Barra de vida vermelha/verde para duelos da Arena.
- `ui_exp_bar_fill`: Barra de experiência azul turquesa brilhante.

### 6.4 Obstaculos da Ilha (Modo de Construcao)
*Dimensoes recomendadas:* 128x128 px (1x1 celula) e 256x192 px (rochedo 2x2). Fundo transparente, base alinhada ao losango do tile.
- `obstacle_bush`: Arbusto verde denso com pequenas flores, ocupa 1 celula.
- `obstacle_rock_small`: Pedra cinzenta musgosa, ocupa 1 celula.
- `obstacle_tree`: Arvore de copa frondosa e tronco grosso, ocupa 1 celula.
- `obstacle_rock_large`: Rochedo grande com veios de cristal, ocupa 2x2 celulas.
- `ui_placement_valid`: (opcional) Moldura de tile verde brilhante para a area livre de construcao.
- `ui_placement_invalid`: (opcional) Moldura de tile vermelha para a area bloqueada.

