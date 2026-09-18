//! Incrusta el icono y los datos de version en los .exe.
//!
//! Tambien incrusta el manifiesto que pide elevacion: sin el, Windows no
//! muestra el escudo del UAC en el icono del instalador y el usuario no sabe
//! que va a pedir permisos hasta que los pide.

fn main() {
    if !cfg!(target_os = "windows") {
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon("recursos/instalador.ico");
    res.set("ProductName", "node-winsvc");
    res.set("FileDescription", "Instalador de servicios de Windows para Node.js");
    res.set("LegalCopyright", "MIT");

    if let Err(e) = res.compile() {
        // No es fatal: sin icono el binario funciona igual.
        println!("cargo:warning=no se pudo incrustar el icono: {e}");
    }

    println!("cargo:rerun-if-changed=recursos/instalador.ico");
}
