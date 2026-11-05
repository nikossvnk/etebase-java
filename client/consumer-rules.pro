# R8 rules for apps using this library. The public Kotlin API needs no rules: apps keep
# what they call. Only code reached reflectively or by name from native code is listed.

# JNA: libjnidispatch looks up JNA classes, fields and methods by name through JNI, and JNA
# reads Structure.FieldOrder annotations reflectively.
-keep class com.sun.jna.** { *; }
-keepattributes RuntimeVisibleAnnotations
# JNA's desktop-only AWT helpers are never loaded on Android.
-dontwarn java.awt.**

# UniFFI bindings (generated into com.etebase.client):
# UniffiLib and IntegrityCheckingUniffiLib bind `external` functions to the Rust exports by
# name (JNA direct mapping), including the classes in their signatures.
-keepclasseswithmembers,includedescriptorclasses class com.etebase.client.** {
    native <methods>;
}
# JNA reads and writes Structure fields by the names in @Structure.FieldOrder and creates
# instances (including the ByValue/ByReference subclasses) reflectively.
-keep class com.etebase.client.** extends com.sun.jna.Structure {
    <fields>;
    <init>(...);
}
# JNA calls the method of a Callback implementation (the Rust -> Kotlin vtables) reflectively.
-keep interface com.etebase.client.** extends com.sun.jna.Callback { *; }
-keepclassmembers class com.etebase.client.** implements com.sun.jna.Callback {
    <methods>;
}
