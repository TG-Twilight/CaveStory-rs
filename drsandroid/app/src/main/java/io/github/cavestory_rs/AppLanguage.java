package io.github.cavestory_rs;

import android.app.LocaleManager;
import android.content.Context;
import android.content.res.Configuration;
import android.content.res.Resources;
import android.os.Build;
import android.os.LocaleList;
import java.util.Locale;

final class AppLanguage {
    static String selection(Context context) {
        LocaleList locales = Resources.getSystem().getConfiguration().getLocales();
        String source = "system:";
        if (Build.VERSION.SDK_INT >= 33) {
            LocaleManager manager = context.getSystemService(LocaleManager.class);
            locales = manager.getSystemLocales();
            if (!manager.getApplicationLocales().isEmpty()) {
                locales = manager.getApplicationLocales();
                source = "app:";
            }
        }
        Locale locale = locales.isEmpty() ? Locale.ENGLISH : locales.get(0);
        return source + GameLanguage.code(locale) + ":" + locale.toLanguageTag();
    }

    static Context localized(Context context) {
        String selection = selection(context);
        String code = selection.split(":", 3)[1];
        Configuration configuration = new Configuration(context.getResources().getConfiguration());
        configuration.setLocales(new LocaleList(Locale.forLanguageTag("jp".equals(code) ? "ja" : code)));
        return context.createConfigurationContext(configuration);
    }
}
