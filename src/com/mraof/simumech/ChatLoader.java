package com.mraof.simumech;

import java.io.File;
import java.io.IOException;
import java.net.MalformedURLException;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.file.FileSystem;

public class ChatLoader extends ClassLoader
{
	@Override
	public Class<?> loadClass(String name) throws ClassNotFoundException 
	{
		return reload(name);
	}
	public static Class reload(String className)
	{
			try {
				URL classPath;
				classPath = new File(System.getProperty("user.dir") + File.separator).toURI().toURL();
				URLClassLoader classLoader = new URLClassLoader(new URL[] {classPath});
				Class cls = classLoader.loadClass(className);
				classLoader.close();
				return cls;
			} catch (Exception e) {e.printStackTrace();}
			return null;
	}
}