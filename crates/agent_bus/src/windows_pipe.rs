//! Per-user local pipe permissions; bearer authentication is an additional boundary.
use anyhow::{ensure, Result};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows::{
    core::{Owned, HSTRING, PWSTR},
    Win32::{
        Foundation::{HANDLE, HLOCAL},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            },
            GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
            TOKEN_USER,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

fn current_user_sid() -> Result<String> {
    let mut token = HANDLE::default();
    // OS-created handles are owned below; neither token contents nor SID are logged.
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;
    }
    let token = unsafe { Owned::new(token) };
    let mut required = 0u32;
    let _ = unsafe { GetTokenInformation(*token, TokenUser, None, 0, &mut required) };
    ensure!(
        required as usize >= std::mem::size_of::<TOKEN_USER>() && required <= 65536,
        "Cannot determine local IPC user"
    );
    // usize storage provides TOKEN_USER alignment; the SID points into this same live allocation.
    let mut data = vec![0usize; (required as usize).div_ceil(std::mem::size_of::<usize>())];
    unsafe {
        GetTokenInformation(
            *token,
            TokenUser,
            Some(data.as_mut_ptr().cast()),
            required,
            &mut required,
        )?;
    }
    let user = unsafe { &*data.as_ptr().cast::<TOKEN_USER>() };
    let mut sid = PWSTR::null();
    unsafe {
        ConvertSidToStringSidW(user.User.Sid, &mut sid)?;
    }
    let _allocation = unsafe { Owned::new(HLOCAL(sid.0.cast())) };
    Ok(unsafe { sid.to_string()? })
}

pub(crate) fn create(endpoint: &str, first: bool) -> Result<NamedPipeServer> {
    let sid = current_user_sid()?;
    let sddl = HSTRING::from(format!("D:P(A;;GA;;;{sid})"));
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(&sddl, 1, &mut descriptor, None)?;
    }
    let _allocation = unsafe { Owned::new(HLOCAL(descriptor.0)) };
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    // CreateNamedPipe copies this descriptor during the call; the owned allocation can then drop.
    let pipe = unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(
                endpoint,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
            )?
    };
    Ok(pipe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use tokio::net::windows::named_pipe::ClientOptions;
    use windows::Win32::Security::{
        Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, GetSecurityInfo, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION,
    };

    #[tokio::test]
    async fn kernel_pipe_dacl_has_only_the_current_user_and_connects_locally() {
        let endpoint = format!(r"\\.\pipe\warp-agent-acl-{}", uuid::Uuid::new_v4());
        let pipe = create(&endpoint, true).unwrap();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            GetSecurityInfo(
                HANDLE(pipe.as_raw_handle()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                None,
                None,
                Some(&mut descriptor),
            )
            .ok()
            .unwrap();
        }
        let _descriptor = unsafe { Owned::new(HLOCAL(descriptor.0)) };
        let mut text = PWSTR::null();
        unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor,
                1,
                DACL_SECURITY_INFORMATION,
                &mut text,
                None,
            )
            .unwrap();
        }
        let _text = unsafe { Owned::new(HLOCAL(text.0.cast())) };
        let text = unsafe { text.to_string().unwrap() };
        assert!(text.contains(&current_user_sid().unwrap()));
        assert!(text.contains("D:P") && text.matches('(').count() == 1);
        assert!(!text.contains(";;;WD") && !text.contains(";;;AN"));
        let client = ClientOptions::new().open(&endpoint).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), pipe.connect())
            .await
            .unwrap()
            .unwrap();
        // The listener must preserve the same permissions for every subsequent instance.
        let next = create(&endpoint, false).unwrap();
        drop(client);
        drop(next);
    }
}
