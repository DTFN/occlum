use super::*;

use super::io_multiplexing::{AsEpollFile, EpollCtlCmd, EpollEventFlags, EpollFile, FdSetExt};
use fs::{CreationFlags, File, FileDesc, FileRef};
use misc::resource_t;
use process::Process;
use std::convert::TryFrom;
use time::timeval_t;
use util::mem_util::from_user;

pub fn do_socket(domain: c_int, socket_type: c_int, protocol: c_int) -> Result<isize> {
    println!(
        "socket: domain: {}, socket_type: 0x{:x}, protocol: {}",
        domain, socket_type, protocol
    );

    let file_ref: Arc<Box<dyn File>> = match domain {
        libc::AF_LOCAL => {
            let unix_socket = UnixSocketFile::new(socket_type, protocol)?;
            Arc::new(Box::new(unix_socket))
        }
        _ => {
            let socket = SocketFile::new(domain, socket_type, protocol)?;
            Arc::new(Box::new(socket))
        }
    };

    let fd = current!().add_file(file_ref, false);
    Ok(fd as isize)
}

pub fn do_connect(
    fd: c_int,
    addr: *const libc::sockaddr,
    addr_len: libc::socklen_t,
) -> Result<isize> {
    println!(
        "connect: fd: {}, addr: {:?}, addr_len: {}",
        fd, addr, addr_len
    );
    // For SOCK_DGRAM sockets not initiated in connection-mode,
    // if address is a null address for the protocol,
    // the socket's peer address shall be reset.
    let need_check: bool = !addr.is_null();
    if need_check {
        from_user::check_array(addr as *const u8, addr_len as usize)?;
    }

    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        if need_check {
            from_user::check_ptr(addr as *const libc::sockaddr_in)?;
        }
        let ret = try_libc!(libc::ocall::connect(socket.fd(), addr, addr_len));
        Ok(ret as isize)
    } else if let Ok(unix_socket) = file_ref.as_unix_socket() {
        let addr = addr as *const libc::sockaddr_un;
        from_user::check_ptr(addr)?;
        let path = from_user::clone_cstring_safely(unsafe { (&*addr).sun_path.as_ptr() })?
            .to_string_lossy()
            .into_owned();
        unix_socket.connect(path)?;
        Ok(0)
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_accept(
    fd: c_int,
    addr: *mut libc::sockaddr,
    addr_len: *mut libc::socklen_t,
) -> Result<isize> {
    do_accept4(fd, addr, addr_len, 0)
}

pub fn do_accept4(
    fd: c_int,
    addr: *mut libc::sockaddr,
    addr_len: *mut libc::socklen_t,
    flags: c_int,
) -> Result<isize> {
    println!(
        "accept4: fd: {}, addr: {:?}, addr_len: {:?}, flags: {:#x}",
        fd, addr, addr_len, flags
    );

    let need_check: bool = !addr.is_null();

    if addr.is_null() ^ addr_len.is_null() {
        return_errno!(EINVAL, "addr and ddr_len should be both null");
    }
    if need_check {
        from_user::check_mut_array(addr as *mut u8, unsafe { *addr_len } as usize)?;
    }

    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        if need_check {
            from_user::check_mut_ptr(addr as *mut libc::sockaddr_in)?;
        }

        let new_socket = socket.accept(addr, addr_len, flags)?;
        let new_file_ref: Arc<Box<dyn File>> = Arc::new(Box::new(new_socket));
        let new_fd = current!().add_file(new_file_ref, false);

        Ok(new_fd as isize)
    } else if let Ok(unix_socket) = file_ref.as_unix_socket() {
        let addr = addr as *mut libc::sockaddr_un;
        if need_check {
            from_user::check_mut_ptr(addr)?;
        }
        // TODO: handle addr
        let new_socket = unix_socket.accept()?;
        let new_file_ref: Arc<Box<dyn File>> = Arc::new(Box::new(new_socket));
        let new_fd = current!().add_file(new_file_ref, false);

        Ok(new_fd as isize)
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_shutdown(fd: c_int, how: c_int) -> Result<isize> {
    println!("shutdown: fd: {}, how: {}", fd, how);
    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        let ret = try_libc!(libc::ocall::shutdown(socket.fd(), how));
        Ok(ret as isize)
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_bind(fd: c_int, addr: *const libc::sockaddr, addr_len: libc::socklen_t) -> Result<isize> {
    println!("bind: fd: {}, addr: {:?}, addr_len: {}", fd, addr, addr_len);
    if addr.is_null() && addr_len == 0 {
        return_errno!(EINVAL, "no address is specified");
    }
    from_user::check_array(addr as *const u8, addr_len as usize)?;

    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        from_user::check_ptr(addr as *const libc::sockaddr_in)?;
        let ret = try_libc!(libc::ocall::bind(socket.fd(), addr, addr_len));
        Ok(ret as isize)
    } else if let Ok(unix_socket) = file_ref.as_unix_socket() {
        let addr = addr as *const libc::sockaddr_un;
        from_user::check_ptr(addr)?;
        let path = from_user::clone_cstring_safely(unsafe { (&*addr).sun_path.as_ptr() })?
            .to_string_lossy()
            .into_owned();
        unix_socket.bind(path)?;
        Ok(0)
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_listen(fd: c_int, backlog: c_int) -> Result<isize> {
    println!("listen: fd: {}, backlog: {}", fd, backlog);
    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        let ret = try_libc!(libc::ocall::listen(socket.fd(), backlog));
        Ok(ret as isize)
    } else if let Ok(unix_socket) = file_ref.as_unix_socket() {
        unix_socket.listen()?;
        Ok(0)
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_setsockopt(
    fd: c_int,
    level: c_int,
    optname: c_int,
    optval: *const c_void,
    optlen: libc::socklen_t,
) -> Result<isize> {
    println!(
        "setsockopt: fd: {}, level: {}, optname: {}, optval: {:?}, optlen: {:?}",
        fd, level, optname, optval, optlen
    );
    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        let ret = try_libc!(libc::ocall::setsockopt(
            socket.fd(),
            level,
            optname,
            optval,
            optlen
        ));
        Ok(ret as isize)
    } else if let Ok(unix_socket) = file_ref.as_unix_socket() {
        warn!("setsockopt for unix socket is unimplemented");
        Ok(0)
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_getsockopt(
    fd: c_int,
    level: c_int,
    optname: c_int,
    optval: *mut c_void,
    optlen: *mut libc::socklen_t,
) -> Result<isize> {
    println!(
        "getsockopt: fd: {}, level: {}, optname: {}, optval: {:?}, optlen: {:?}",
        fd, level, optname, optval, optlen
    );
    let file_ref = current!().file(fd as FileDesc)?;
    let socket = file_ref.as_socket()?;

    let ret = try_libc!(libc::ocall::getsockopt(
        socket.fd(),
        level,
        optname,
        optval,
        optlen
    ));
    Ok(ret as isize)
}

pub fn do_getpeername(
    fd: c_int,
    addr: *mut libc::sockaddr,
    addr_len: *mut libc::socklen_t,
) -> Result<isize> {
    println!(
        "getpeername: fd: {}, addr: {:?}, addr_len: {:?}",
        fd, addr, addr_len
    );
    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        let ret = try_libc!(libc::ocall::getpeername(socket.fd(), addr, addr_len));
        Ok(ret as isize)
    } else if let Ok(unix_socket) = file_ref.as_unix_socket() {
        warn!("getpeername for unix socket is unimplemented");
        return_errno!(
            ENOTCONN,
            "hack for php: Transport endpoint is not connected"
        )
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_getsockname(
    fd: c_int,
    addr: *mut libc::sockaddr,
    addr_len: *mut libc::socklen_t,
) -> Result<isize> {
    println!(
        "getsockname: fd: {}, addr: {:?}, addr_len: {:?}",
        fd, addr, addr_len
    );
    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        let ret = try_libc!(libc::ocall::getsockname(socket.fd(), addr, addr_len));
        Ok(ret as isize)
    } else if let Ok(unix_socket) = file_ref.as_unix_socket() {
        warn!("getsockname for unix socket is unimplemented");
        Ok(0)
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_sendto(
    fd: c_int,
    base: *const c_void,
    len: size_t,
    flags: c_int,
    addr: *const libc::sockaddr,
    addr_len: libc::socklen_t,
) -> Result<isize> {
    println!(
        "sendto: fd: {}, base: {:?}, len: {}, flags: {} addr: {:?}, addr_len: {}",
        fd, base, len, flags, addr, addr_len
    );
    from_user::check_array(base as *const u8, len)?;

    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        // TODO: check addr and addr_len according to connection mode
        let ret = try_libc!(libc::ocall::sendto(
            socket.fd(),
            base,
            len,
            flags,
            addr,
            addr_len
        ));
        Ok(ret as isize)
    } else if let Ok(unix) = file_ref.as_unix_socket() {
        if !addr.is_null() || addr_len != 0 {
            return_errno!(EISCONN, "Only connection-mode socket is supported");
        }

        if !unix.is_connected() {
            return_errno!(ENOTCONN, "the socket has not been connected yet");
        }

        let data = unsafe { std::slice::from_raw_parts(base as *const u8, len) };
        unix.write(data).map(|u| u as isize)
    } else {
        return_errno!(EBADF, "unsupported file type");
    }
}

pub fn do_recvfrom(
    fd: c_int,
    base: *mut c_void,
    len: size_t,
    flags: c_int,
    addr: *mut libc::sockaddr,
    addr_len: *mut libc::socklen_t,
) -> Result<isize> {
    println!(
        "recvfrom: fd: {}, base: {:?}, len: {}, flags: {}, addr: {:?}, addr_len: {:?}",
        fd, base, len, flags, addr, addr_len
    );
    let file_ref = current!().file(fd as FileDesc)?;
    let socket = file_ref.as_socket()?;

    let ret = try_libc!(libc::ocall::recvfrom(
        socket.fd(),
        base,
        len,
        flags,
        addr,
        addr_len
    ));
    Ok(ret as isize)
}

pub fn do_socketpair(
    domain: c_int,
    socket_type: c_int,
    protocol: c_int,
    sv: *mut c_int,
) -> Result<isize> {
    println!(
        "socketpair: domain: {}, type:0x{:x}, protocol: {}",
        domain, socket_type, protocol
    );
    let mut sock_pair = unsafe {
        from_user::check_mut_array(sv, 2)?;
        std::slice::from_raw_parts_mut(sv as *mut u32, 2)
    };

    if (domain == libc::AF_UNIX) {
        let (client_socket, server_socket) =
            UnixSocketFile::socketpair(socket_type as i32, protocol as i32)?;
        let current = current!();
        let mut files = current.files().lock().unwrap();
        sock_pair[0] = files.put(Arc::new(Box::new(client_socket)), false);
        sock_pair[1] = files.put(Arc::new(Box::new(server_socket)), false);

        println!("socketpair: ({}, {})", sock_pair[0], sock_pair[1]);
        Ok(0)
    } else if (domain == libc::AF_TIPC) {
        return_errno!(EAFNOSUPPORT, "cluster domain sockets not supported")
    } else {
        return_errno!(EAFNOSUPPORT, "domain not supported")
    }
}

pub fn do_sendmsg(fd: c_int, msg_ptr: *const msghdr, flags_c: c_int) -> Result<isize> {
    println!(
        "sendmsg: fd: {}, msg: {:?}, flags: 0x{:x}",
        fd, msg_ptr, flags_c
    );

    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        let msg_c = {
            from_user::check_ptr(msg_ptr)?;
            let msg_c = unsafe { &*msg_ptr };
            msg_c.check_member_ptrs()?;
            msg_c
        };
        let msg = unsafe { MsgHdr::from_c(&msg_c)? };

        let flags = SendFlags::from_bits_truncate(flags_c);

        socket
            .sendmsg(&msg, flags)
            .map(|bytes_sent| bytes_sent as isize)
    } else if let Ok(socket) = file_ref.as_unix_socket() {
        return_errno!(EBADF, "does not support unix socket")
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

pub fn do_recvmsg(fd: c_int, msg_mut_ptr: *mut msghdr_mut, flags_c: c_int) -> Result<isize> {
    println!(
        "recvmsg: fd: {}, msg: {:?}, flags: 0x{:x}",
        fd, msg_mut_ptr, flags_c
    );

    let file_ref = current!().file(fd as FileDesc)?;
    if let Ok(socket) = file_ref.as_socket() {
        let msg_mut_c = {
            from_user::check_mut_ptr(msg_mut_ptr)?;
            let msg_mut_c = unsafe { &mut *msg_mut_ptr };
            msg_mut_c.check_member_ptrs()?;
            msg_mut_c
        };
        let mut msg_mut = unsafe { MsgHdrMut::from_c(msg_mut_c)? };

        let flags = RecvFlags::from_bits_truncate(flags_c);

        socket
            .recvmsg(&mut msg_mut, flags)
            .map(|bytes_recvd| bytes_recvd as isize)
    } else if let Ok(socket) = file_ref.as_unix_socket() {
        return_errno!(EBADF, "does not support unix socket")
    } else {
        return_errno!(EBADF, "not a socket")
    }
}

#[allow(non_camel_case_types)]
trait c_msghdr_ext {
    fn check_member_ptrs(&self) -> Result<()>;
}

impl c_msghdr_ext for msghdr {
    // TODO: implement this!
    fn check_member_ptrs(&self) -> Result<()> {
        Ok(())
    }
    /*
            ///user space check
            pub unsafe fn check_from_user(user_hdr: *const msghdr) -> Result<()> {
                Self::check_pointer(user_hdr, from_user::check_ptr)
            }

            ///Check msghdr ptr
            pub unsafe fn check_pointer(
                user_hdr: *const msghdr,
                check_ptr: fn(*const u8) -> Result<()>,
            ) -> Result<()> {
                check_ptr(user_hdr as *const u8)?;

                if (*user_hdr).msg_name.is_null() ^ ((*user_hdr).msg_namelen == 0) {
                    return_errno!(EINVAL, "name length is invalid");
                }

                if (*user_hdr).msg_iov.is_null() ^ ((*user_hdr).msg_iovlen == 0) {
                    return_errno!(EINVAL, "iov length is invalid");
                }

                if (*user_hdr).msg_control.is_null() ^ ((*user_hdr).msg_controllen == 0) {
                    return_errno!(EINVAL, "control length is invalid");
                }

                if !(*user_hdr).msg_name.is_null() {
                    check_ptr((*user_hdr).msg_name as *const u8)?;
                }

                if !(*user_hdr).msg_iov.is_null() {
                    check_ptr((*user_hdr).msg_iov as *const u8)?;
                    let iov_slice = slice::from_raw_parts((*user_hdr).msg_iov, (*user_hdr).msg_iovlen);
                    for iov in iov_slice {
                        check_ptr(iov.iov_base as *const u8)?;
                    }
                }

                if !(*user_hdr).msg_control.is_null() {
                    check_ptr((*user_hdr).msg_control as *const u8)?;
                }
                Ok(())
            }
    */
}

impl c_msghdr_ext for msghdr_mut {
    fn check_member_ptrs(&self) -> Result<()> {
        Ok(())
    }
}

pub fn do_select(
    nfds: c_int,
    readfds: *mut libc::fd_set,
    writefds: *mut libc::fd_set,
    exceptfds: *mut libc::fd_set,
    timeout: *mut timeval_t,
) -> Result<isize> {
    // check arguments
    let soft_rlimit_nofile = current!()
        .rlimits()
        .lock()
        .unwrap()
        .get(resource_t::RLIMIT_NOFILE)
        .get_cur();
    if nfds < 0 || nfds > libc::FD_SETSIZE as i32 || nfds as u64 > soft_rlimit_nofile {
        return_errno!(
            EINVAL,
            "nfds is negative or exceeds the resource limit or FD_SETSIZE"
        );
    }

    if !timeout.is_null() {
        from_user::check_ptr(timeout)?;
        unsafe {
            (*timeout).validate()?;
        }
    }

    // Select handles empty set and null in the same way
    // TODO: Elegently handle the empty fd_set without allocating redundant fd_set
    let mut empty_set_for_read = libc::fd_set::new_empty();
    let mut empty_set_for_write = libc::fd_set::new_empty();
    let mut empty_set_for_except = libc::fd_set::new_empty();

    let readfds = if !readfds.is_null() {
        from_user::check_mut_ptr(readfds)?;
        unsafe { &mut *readfds }
    } else {
        &mut empty_set_for_read
    };
    let writefds = if !writefds.is_null() {
        from_user::check_mut_ptr(writefds)?;
        unsafe { &mut *writefds }
    } else {
        &mut empty_set_for_write
    };
    let exceptfds = if !exceptfds.is_null() {
        from_user::check_mut_ptr(exceptfds)?;
        unsafe { &mut *exceptfds }
    } else {
        &mut empty_set_for_except
    };

    let ret = io_multiplexing::select(nfds, readfds, writefds, exceptfds, timeout)?;
    Ok(ret)
}

pub fn do_poll(fds: *mut PollEvent, nfds: libc::nfds_t, timeout: c_int) -> Result<isize> {
    // It behaves like sleep when fds is null and nfds is zero.
    if !fds.is_null() || nfds != 0 {
        from_user::check_mut_array(fds, nfds as usize)?;
    }

    let soft_rlimit_nofile = current!()
        .rlimits()
        .lock()
        .unwrap()
        .get(resource_t::RLIMIT_NOFILE)
        .get_cur();
    // TODO: Check nfds against the size of the stack used in ocall to prevent stack overflow
    if nfds > soft_rlimit_nofile {
        return_errno!(EINVAL, "The nfds value exceeds the RLIMIT_NOFILE value.");
    }

    let polls = unsafe { std::slice::from_raw_parts_mut(fds, nfds as usize) };
    println!("poll: {:?}, timeout: {}", polls, timeout);

    let mut time_val = timeval_t::new(
        ((timeout as u32) / 1000) as i64,
        ((timeout as u32) % 1000 * 1000) as i64,
    );
    let tmp_to = if timeout == -1 {
        std::ptr::null_mut()
    } else {
        &mut time_val
    };

    let n = io_multiplexing::do_poll(polls, tmp_to)?;
    Ok(n as isize)
}

pub fn do_epoll_create(size: c_int) -> Result<isize> {
    if size <= 0 {
        return_errno!(EINVAL, "size is not positive");
    }
    do_epoll_create1(0)
}

pub fn do_epoll_create1(raw_flags: c_int) -> Result<isize> {
    // Only O_CLOEXEC is valid
    let flags = CreationFlags::from_bits(raw_flags as u32)
        .ok_or_else(|| errno!(EINVAL, "invalid flags"))?
        & CreationFlags::O_CLOEXEC;
    let epoll_file = io_multiplexing::EpollFile::new(flags)?;
    let file_ref: Arc<Box<dyn File>> = Arc::new(Box::new(epoll_file));
    let close_on_spawn = flags.contains(CreationFlags::O_CLOEXEC);
    let fd = current!().add_file(file_ref, close_on_spawn);

    Ok(fd as isize)
}

pub fn do_epoll_ctl(
    epfd: c_int,
    op: c_int,
    fd: c_int,
    event: *const libc::epoll_event,
) -> Result<isize> {
    println!("epoll_ctl: epfd: {}, op: {:?}, fd: {}", epfd, op, fd);
    let inner_event = if !event.is_null() {
        from_user::check_ptr(event)?;
        Some(EpollEvent::from_raw(unsafe { &*event })?)
    } else {
        None
    };

    let epfile_ref = current!().file(epfd as FileDesc)?;
    let epoll_file = epfile_ref.as_epfile()?;

    epoll_file.control(
        EpollCtlCmd::try_from(op)?,
        fd as FileDesc,
        inner_event.as_ref(),
    )?;
    Ok(0)
}

pub fn do_epoll_wait(
    epfd: c_int,
    events: *mut libc::epoll_event,
    max_events: c_int,
    timeout: c_int,
) -> Result<isize> {
    let max_events = {
        if max_events <= 0 {
            return_errno!(EINVAL, "maxevents <= 0");
        }
        max_events as usize
    };
    let raw_events = {
        from_user::check_mut_array(events, max_events)?;
        unsafe { std::slice::from_raw_parts_mut(events, max_events) }
    };

    // A new vector to store EpollEvent, which may degrade the performance due to extra copy.
    let mut inner_events: Vec<EpollEvent> =
        vec![EpollEvent::new(EpollEventFlags::empty(), 0); max_events];

    println!(
        "epoll_wait: epfd: {}, len: {:?}, timeout: {}",
        epfd,
        raw_events.len(),
        timeout
    );

    let epfile_ref = current!().file(epfd as FileDesc)?;
    let epoll_file = epfile_ref.as_epfile()?;

    let count = epoll_file.wait(&mut inner_events, timeout)?;

    for i in 0..count {
        raw_events[i] = inner_events[i].to_raw();
    }

    Ok(count as isize)
}

pub fn do_epoll_pwait(
    epfd: c_int,
    events: *mut libc::epoll_event,
    maxevents: c_int,
    timeout: c_int,
    sigmask: *const usize, //TODO:add sigset_t
) -> Result<isize> {
    if !sigmask.is_null() {
        warn!("epoll_pwait cannot handle signal mask, yet");
    } else {
        info!("epoll_wait");
    }
    do_epoll_wait(epfd, events, maxevents, timeout)
}
