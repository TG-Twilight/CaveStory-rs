package io.github.cavestory_rs;

import java.util.Locale;

/** The three complete resource languages currently shipped by CaveStory-rs. */
final class GameLanguage {
    static String code(Locale locale) {
        if ("ja".equals(locale.getLanguage())) return "jp";
        if ("zh".equals(locale.getLanguage())) {
            if ("Hant".equals(locale.getScript())) return "en";
            if ("Hans".equals(locale.getScript())) return "zh-Hans";
            switch (locale.getCountry()) {
                case "": case "CN": case "SG": case "MY": return "zh-Hans";
                default: return "en";
            }
        }
        return "en";
    }
}
