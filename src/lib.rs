// This is the interface to the JVM that we'll
// call the majority of our methods on.
use jni::JNIEnv;

// These objects are what you should use as arguments to your native function.
// They carry extra lifetime information to prevent them escaping this context
// and getting used after being GC'd.
use jni::objects::{GlobalRef, JClass, JObject, JString, JValue, JByteBuffer};

// This is just a pointer. We'll be returning it from our function.
// We can't return one of the objects with lifetime information because the
// lifetime checker won't let us.
use jni::sys::{jbyteArray, jint, jlong, jstring};

use std::{sync::mpsc, thread, time::Duration, env};
use std::mem::forget;
use jni::signature::{JavaType, Primitive};
use pnet::datalink;
use pnet::datalink::{
	DataLinkSender,
	DataLinkReceiver,
	NetworkInterface,
	Channel::Ethernet
};
use pnet::packet::ethernet::EthernetPacket;

#[no_mangle]
pub extern "system" fn Java_rio_github_cellularghost_RawSock_init(
    env: JNIEnv,
    this: JClass
) {
	let desc = env.get_field_id(this, "self", "J").unwrap();
	env.set_field_unchecked(this, desc, JValue::Long(Box::into_raw(Box::new(RawSock::new())) as i64));
}


// This keeps rust from "mangling" the name and making it unique for this crate.
#[no_mangle]
pub unsafe extern "system" fn Java_io_github_cellularghost_RawSock_read(
    env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    class: JClass,
	buffer: jbyteArray,
	start: jint
) -> jint {
	let desc = env.get_field_id(class, "self", "J").unwrap();
	let ptr = env.get_field_unchecked(class, desc, JavaType::Primitive(Primitive::Long)).unwrap();
	let mut raw_sock = Box::from_raw(ptr.j().unwrap() as *mut RawSock);
	return match raw_sock.rx.next() {
		Ok(packet) => {
			let a = unsafe {
				&*(packet as *const [u8] as *const [i8])
			};
			env.set_byte_array_region(buffer, start, a);
			packet.len()
		},
		Err(e) => {
			env.throw_new("java.io.IOException", e.to_string());
			0
		}
	} as i32;
}

#[no_mangle]
pub unsafe extern "system" fn Java_io_github_cellularghost_RawSock_write(
	env: JNIEnv,
	// this is the class that owns our
	// static method. Not going to be
	// used, but still needs to have
	// an argument slot
	class: JClass,
	buffer: jbyteArray,
	start: jint,
	len: jint
) {
	let desc = env.get_field_id(class, "self", "J").unwrap();
	let ptr = env.get_field_unchecked(class, desc, JavaType::Primitive(Primitive::Long)).unwrap();
	let mut raw_sock = Box::from_raw(ptr.j().unwrap() as *mut RawSock);
	raw_sock.tx.build_and_send(1, len as usize, &mut |__buffer| {
		let a = unsafe {
			&mut *(__buffer as *mut [u8] as *mut [i8])
		};
		env.get_byte_array_region(buffer, start, a).unwrap()
	});
}

#[no_mangle]
pub unsafe extern "system" fn Java_io_github_cellularghost_RawSock_close(
    env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    class: JClass,
) {
    let desc = env.get_field_id(class, "self", "J").unwrap();
    let ptr = env.get_field_unchecked(class, desc, JavaType::Primitive(Primitive::Long)).unwrap();
    let raw_sock = Box::from_raw(ptr.j().unwrap() as *mut RawSock);
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}

struct RawSock {
	tx: Box<dyn DataLinkSender + 'static>,
	rx: Box<dyn DataLinkReceiver + 'static>
}

impl RawSock {
	pub fn new() -> Self{
		let interface_name = env::args().nth(1).unwrap();
		let interface_names_match =
			|iface: &NetworkInterface| iface.name == interface_name;

		// Find the network interface with the provided name
		let interfaces = datalink::interfaces();
		let interface = interfaces.into_iter()
			.filter(|iface: &NetworkInterface| iface.name == interface_name)
			.next()
			.unwrap();

		let mut config: datalink::Config = Default::default();
		config.read_timeout.replace(Duration::new(1, 0));

		let (mut tx, mut rx) = match datalink::channel(&interface, config) {
			Ok(Ethernet(tx, rx)) => (tx, rx),
			Ok(_) => panic!("Unhandled channel type"),
			Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
		};
		Self {
			tx,
			rx
		}
	}
}