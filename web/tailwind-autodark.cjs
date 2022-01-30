const plugin = require('tailwindcss/plugin');

const isSvench = Boolean(process.env.SVENCH);
const defaultBrightnessValues = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900];

exports.autoDarkPlugin = function autoDarkPlugin({ mainElement, colors, brightnessValues }) {
  brightnessValues = brightnessValues ?? defaultBrightnessValues;
  return plugin(({ addBase, theme }) => {
    const main = isSvench ? ".svench-body" : mainElement;

    function generateLightDark(colorSource) {
      let lightColorValues = brightnessValues.map((color) => theme(`colors.${colorSource}.${color}`));
      let lightColors = Object.fromEntries(
        brightnessValues.map((color, i) => [`--color-d${colorSource}-${color}`, lightColorValues[i]])
      );

      let darkColorValues = lightColorValues.slice().reverse();
      let darkColors = Object.fromEntries(
        brightnessValues.map((color, i) => [`--color-d${colorSource}-${color}`, darkColorValues[i]])
      );

      return {
        light: lightColors,
        dark: darkColors,
      };
    }

    let lightColors = {
      "--color-dwhite": "white",
      "--color-dblack": "black",
    };

    let darkColors = {
      "--color-dwhite": "black",
      "--color-dblack": "white",
    };

    for(let color of colors) {
      let generated = generateLightDark(color);
      Object.assign(lightColors, generated.light);
      Object.assign(darkColors, generated.dark);
    }

    addBase({
      [main]: lightColors,
      [`${main}.dark`]: darkColors,
    });
  });
}

exports.autoDarkColors = function autoDarkColors({ colors, brightnessValues }) {
  brightnessValues = brightnessValues ?? defaultBrightnessValues;
  let output = Object.fromEntries(
    colors.map((color) => {
      let colorName = "d" + color;
      return [colorName, Object.fromEntries(brightnessValues.map((c) => [c, `var(--color-${colorName}-${c})`]))];
    })
  );

  output.dblack = `var(--color-dblack)`;
  output.dwhite = `var(--color-dwhite)`;

  return output;
}
