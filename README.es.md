<p align="center"><img src="src-tauri/icons/icon.svg" width="96" alt=""></p>

<h1 align="center">ZomboLens</h1>

<p align="center">El estado de tu personaje de <b>Project Zomboid</b> en un overlay pequeño, leído de la red.<br>
<a href="README.md">Read in English</a></p>

---

ZomboLens te enseña lo que te dicen los moodles y el panel de salud del juego
—hambre, sed, aguante, dolor, heridas, vendas, nutrición…— con números y
barras en un overlay que colocas donde quieras, y tus coordenadas tal y como
las escriben los admins.

**Solo lee el tráfico de red que tu juego recibe y envía.** No toca el juego,
no lee su memoria, no necesita mods y funciona en servidores vanilla.

## Lo que *no* muestra, a propósito

El servidor también manda cosas que el juego te oculta adrede, como tu nivel
exacto de infección zombi. **ZomboLens nunca las lee**: el decodificador se
las salta, no solo se esconden en pantalla. Muestra lo que ya podrías saber
con la interfaz del juego, solo que con más precisión y todo junto.

## Qué muestra

| Modo | |
|---|---|
| **Básico** | Salud, hambre, sed, sueño, aguante, estrés, pánico, dolor, aburrimiento, infelicidad, mojado, temperatura y tu posición |
| **Ampliado** | Todas las estadísticas (también ira, moral, cordura, forma física, borrachera, mono de nicotina, enfermedad, intoxicación alimentaria, veneno…), nutrición (calorías, proteínas, grasas, carbohidratos, peso), carga, velocidad al andar y correr, resfriado y medicamentos activos |
| **Total** | Además, las heridas por parte del cuerpo (sangrado, mordiscos, arañazos, cortes, heridas profundas, fracturas, quemaduras, cristales, balas, vida de la venda, puntos, férulas) y los moodles estimados |

La salud total se aproxima a partir de las 17 partes del cuerpo (el juego no
la manda). Los moodles se estiman con los umbrales de la wiki.

Se minimiza a un círculo (que se pone rojo mientras sangras) y se arrastra
donde quieras.

## Instalación

- **Linux:** paquetes `.deb` y `.rpm` en [Releases](../../releases), o en Arch/CachyOS `cd packaging/arch && makepkg -si`. Los paquetes dan el permiso de captura (`CAP_NET_RAW`); nunca se ejecuta como root.
- **Windows:** instala **[Npcap](https://npcap.com/#download)** marcando *«Install Npcap in WinPcap API-compatible Mode»* y después el instalador de [Releases](../../releases).
- **Hyprland:** la regla de ventana está en el [README en inglés](README.md#linux). El overlay sigue al juego: aparece en su escritorio, encima de la pantalla completa, y se esconde en los demás.

## Compatibilidad

Hecho y probado para la **build 42.21**. Los números de tipo de paquete
pueden moverse cuando el juego añade paquetes; si un parche lo rompe, hay que
actualizarlos en [`src-tauri/src/pz.rs`](src-tauri/src/pz.rs).

## ¿Está permitido?

ZomboLens es pasivo y solo enseña lo que el juego ya te deja ver. Aun así,
cada servidor tiene sus normas: revísalas y úsalo bajo tu responsabilidad.

## Apoyar el proyecto

Si te resulta útil y quieres invitarme a un café, se aceptan donaciones en ETH (red principal de Ethereum):

```text
0x334B54cE55d40A87919b4066102279D70865b1ae
```

## Licencia

GPL-3.0. Project Zomboid es una marca de The Indie Stone; este proyecto no tiene relación con ellos.
